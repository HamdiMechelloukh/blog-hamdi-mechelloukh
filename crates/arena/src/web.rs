//! Glue avec la page /jeu : atelier HTML (règles, bonus), boucle de combat et rendu.
//! Les textes viennent du JSON `#arena-i18n` rendu par le site, dans la langue de la page.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, HtmlCanvasElement, HtmlSelectElement};

use crate::ai::opponent;
use crate::loadout::{Bonus, Loadout, Weapon};
use crate::program::{Action, Condition, Rule};
use crate::render::Renderer;
use crate::rng::Rng;
use crate::sim::{Fight, Outcome, PLAYER, STEP};

/// Rattrapage maximal par frame : après un ralentissement, la simulation prend du retard plutôt que de sauter.
const MAX_STEPS_PER_FRAME: u32 = 5;
/// dt borné : après un onglet caché, pas de saut.
const MAX_FRAME_TIME: f32 = 0.1;

const STARTER_PROGRAM: [Option<Rule>; 2] = [
    Some(Rule { condition: Condition::EnemyInRange, action: Action::Fire }),
    Some(Rule { condition: Condition::Always, action: Action::Advance }),
];

/// Callback requestAnimationFrame qui se re-programme lui-même.
type FrameLoop = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

enum Phase {
    Workshop,
    Fighting,
    Reward([Bonus; 2]),
    Over,
}

struct Labels(JsValue);

impl Labels {
    fn get(&self, key: &str) -> String {
        js_sys::Reflect::get(&self.0, &key.into()).ok().and_then(|value| value.as_string()).unwrap_or_else(|| key.into())
    }

    /// Libellé avec `{}` remplacé par `value`.
    fn fill(&self, key: &str, value: &str) -> String {
        self.get(key).replace("{}", value)
    }
}

struct Page {
    workshop: Element,
    rules: Element,
    status: Element,
    loadouts: Element,
    reward: Element,
    bonuses: Element,
    over: Element,
    score: Element,
}

struct Game {
    page: Page,
    labels: Labels,
    renderer: Renderer,
    rng: Rng,
    wins: u32,
    loadout: Loadout,
    /// Emplacements de l'atelier, vides compris, dans l'ordre de l'écran.
    slots: Vec<Option<Rule>>,
    /// Emplacement de chaque règle du programme en combat : sert à surligner la règle active.
    slot_of_rule: Vec<usize>,
    highlighted: Option<usize>,
    opponent: (Loadout, Vec<Rule>),
    fight: Fight,
    phase: Phase,
    accumulator: f32,
    last_time: Option<f64>,
}

#[wasm_bindgen(start)]
fn start() {
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = run().await {
            web_sys::console::warn_1(&error);
        }
    });
}

async fn run() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("pas de window")?;
    let document = window.document().ok_or("pas de document")?;
    let labels = Labels(js_sys::JSON::parse(&element(&document, "arena-i18n")?.text_content().unwrap_or_default())?);
    let page = Page {
        workshop: element(&document, "arena-workshop")?,
        rules: element(&document, "arena-rules")?,
        status: element(&document, "arena-status")?,
        loadouts: element(&document, "arena-loadouts")?,
        reward: element(&document, "arena-reward")?,
        bonuses: element(&document, "arena-bonuses")?,
        over: element(&document, "arena-over")?,
        score: element(&document, "arena-score")?,
    };
    let canvas: HtmlCanvasElement = element(&document, "arena-canvas")?.dyn_into()?;
    let mut rng = Rng::new(js_sys::Date::now() as u64);
    let renderer = match Renderer::new(canvas, rng.next_u64()).await {
        Ok(renderer) => renderer,
        Err(error) => {
            page.status.set_text_content(Some(&labels.get("unsupported")));
            page.workshop.toggle_attribute_with_force("disabled", true)?;
            return Err(error.into());
        }
    };
    let opponent = opponent(0, &mut rng);
    let game = Rc::new(RefCell::new(Game {
        fight: Fight::new((Loadout::starter(), Vec::new()), opponent.clone(), 0),
        page,
        labels,
        renderer,
        rng,
        wins: 0,
        loadout: Loadout::starter(),
        slots: Vec::new(),
        slot_of_rule: Vec::new(),
        highlighted: None,
        opponent,
        phase: Phase::Workshop,
        accumulator: 0.0,
        last_time: None,
    }));
    game.borrow_mut().new_run();

    let handler = game.clone();
    on_click(&element(&document, "arena-fight")?, move |_| handler.borrow_mut().start_fight())?;
    let handler = game.clone();
    on_click(&element(&document, "arena-restart")?, move |_| handler.borrow_mut().new_run())?;
    let handler = game.clone();
    on_click(&game.borrow().page.rules, move |event| {
        if let Some(button) = closest(&event, "[data-move]") {
            let slot = button.get_attribute("data-slot").and_then(|slot| slot.parse().ok());
            let up = button.get_attribute("data-move").as_deref() == Some("up");
            if let Some(slot) = slot {
                handler.borrow_mut().move_slot(slot, up);
            }
        }
    })?;
    let handler = game.clone();
    on_click(&game.borrow().page.bonuses, move |event| {
        if let Some(choice) = closest(&event, "[data-bonus]").and_then(|button| button.get_attribute("data-bonus")) {
            handler.borrow_mut().choose(usize::from(choice == "1"));
        }
    })?;

    // Boucle requestAnimationFrame : elle se suspend d'elle-même quand l'onglet est caché.
    let frame: FrameLoop = Rc::new(RefCell::new(None));
    let next_frame = frame.clone();
    let loop_window = window.clone();
    *frame.borrow_mut() = Some(Closure::new(move |now: f64| {
        game.borrow_mut().frame(now);
        if let Some(callback) = next_frame.borrow().as_ref() {
            let _ = loop_window.request_animation_frame(callback.as_ref().unchecked_ref());
        }
    }));
    window.request_animation_frame(frame.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;
    Ok(())
}

impl Game {
    fn new_run(&mut self) {
        self.wins = 0;
        self.loadout = Loadout::starter();
        self.slots = STARTER_PROGRAM.to_vec();
        self.slots.resize(self.loadout.rule_slots, None);
        self.prepare_fight();
    }

    /// Nouvel adversaire, montré à l'arrêt dans l'arène pendant que le joueur programme.
    fn prepare_fight(&mut self) {
        self.opponent = opponent(self.wins, &mut self.rng);
        self.fight = Fight::new((self.loadout.clone(), self.program()), self.opponent.clone(), self.rng.next_u64());
        self.phase = Phase::Workshop;
        self.highlight(None);
        self.render_rules();
        let describe_robots = format!(
            "{}<br>{}",
            self.labels.fill("you", &self.describe(&self.loadout)),
            self.labels.fill("opponent", &self.describe(&self.opponent.0)),
        );
        self.page.loadouts.set_inner_html(&describe_robots);
        self.page.status.set_text_content(Some(&self.labels.fill("round", &(self.wins + 1).to_string())));
        let _ = self.page.workshop.toggle_attribute_with_force("disabled", false);
        let _ = self.page.reward.toggle_attribute_with_force("hidden", true);
        let _ = self.page.over.toggle_attribute_with_force("hidden", true);
    }

    fn start_fight(&mut self) {
        if !matches!(self.phase, Phase::Workshop) {
            return;
        }
        self.read_slots();
        self.fight = Fight::new((self.loadout.clone(), self.program()), self.opponent.clone(), self.rng.next_u64());
        self.phase = Phase::Fighting;
        self.accumulator = 0.0;
        self.page.status.set_text_content(Some(&self.labels.get("fighting")));
        let _ = self.page.workshop.toggle_attribute_with_force("disabled", true);
    }

    fn frame(&mut self, now: f64) {
        let dt = self.last_time.map_or(0.0, |last| ((now - last) / 1000.0) as f32).min(MAX_FRAME_TIME);
        self.last_time = Some(now);
        if matches!(self.phase, Phase::Fighting) {
            self.accumulator += dt;
            let mut steps = 0;
            while self.accumulator >= STEP && steps < MAX_STEPS_PER_FRAME {
                self.fight.step();
                self.renderer.spawn_effects(&self.fight.events);
                self.accumulator -= STEP;
                steps += 1;
            }
            let active = self.fight.robots[PLAYER].active_rule.map(|rule| self.slot_of_rule[rule]);
            self.highlight(active);
            if let Some(outcome) = self.fight.outcome() {
                self.finish(outcome);
            }
        }
        self.renderer.draw(&self.fight, dt);
    }

    fn finish(&mut self, outcome: Outcome) {
        self.highlight(None);
        match outcome {
            Outcome::Won => {
                self.wins += 1;
                let offer = self.loadout.offer(&mut self.rng);
                let buttons: String = offer
                    .iter()
                    .enumerate()
                    .map(|(index, bonus)| {
                        format!(
                            r#"<button type="button" class="btn btn-outline arena-bonus" data-bonus="{index}">{}</button>"#,
                            self.bonus_label(*bonus)
                        )
                    })
                    .collect();
                self.page.bonuses.set_inner_html(&buttons);
                self.page.status.set_text_content(Some(&self.labels.get("won")));
                let _ = self.page.reward.toggle_attribute_with_force("hidden", false);
                self.phase = Phase::Reward(offer);
            }
            Outcome::Lost => {
                self.page.status.set_text_content(Some(&self.labels.get("lost")));
                self.page.score.set_text_content(Some(&self.labels.fill("score", &self.wins.to_string())));
                let _ = self.page.over.toggle_attribute_with_force("hidden", false);
                self.phase = Phase::Over;
            }
        }
    }

    fn choose(&mut self, choice: usize) {
        let Phase::Reward(offer) = self.phase else {
            return;
        };
        self.loadout.apply(offer[choice]);
        self.slots.resize(self.loadout.rule_slots, None);
        self.prepare_fight();
    }

    fn move_slot(&mut self, slot: usize, up: bool) {
        let target = if up { slot.checked_sub(1) } else { Some(slot + 1).filter(|&next| next < self.slots.len()) };
        if let Some(target) = target {
            self.read_slots();
            self.slots.swap(slot, target);
            self.render_rules();
        }
    }

    /// Règles non vides dans l'ordre des emplacements ; retient l'emplacement de chacune.
    fn program(&mut self) -> Vec<Rule> {
        let (slots, rules) = self.slots.iter().enumerate().filter_map(|(slot, rule)| Some((slot, (*rule)?))).unzip();
        self.slot_of_rule = slots;
        rules
    }

    fn read_slots(&mut self) {
        let Ok(rows) = self.page.rules.query_selector_all("li") else {
            return;
        };
        let value = |row: &Element, selector: &str| {
            row.query_selector(selector).ok().flatten().and_then(|select| select.dyn_into::<HtmlSelectElement>().ok()).map(|select| select.value())
        };
        self.slots = (0..rows.length())
            .filter_map(|index| rows.item(index)?.dyn_into::<Element>().ok())
            .map(|row| {
                let condition = Condition::from_id(&value(&row, ".arena-condition")?)?;
                let action = Action::from_id(&value(&row, ".arena-action")?)?;
                Some(Rule { condition, action })
            })
            .collect();
    }

    fn render_rules(&self) {
        let actions = self.loadout.actions();
        let last = self.slots.len().saturating_sub(1);
        let rows: String = self
            .slots
            .iter()
            .enumerate()
            .map(|(slot, rule)| {
                let condition_options: String = std::iter::once(format!(r#"<option value="">{}</option>"#, self.labels.get("empty")))
                    .chain(self.loadout.conditions.iter().map(|condition| {
                        let selected = if rule.is_some_and(|rule| rule.condition == *condition) { " selected" } else { "" };
                        format!(
                            r#"<option value="{}"{selected}>{}</option>"#,
                            condition.id(),
                            self.labels.get(&format!("condition.{}", condition.id()))
                        )
                    }))
                    .collect();
                let action_options: String = actions
                    .iter()
                    .map(|action| {
                        let selected = if rule.is_some_and(|rule| rule.action == *action) { " selected" } else { "" };
                        format!(
                            r#"<option value="{}"{selected}>{}</option>"#,
                            action.id(),
                            self.labels.get(&format!("action.{}", action.id()))
                        )
                    })
                    .collect();
                format!(
                    r#"<li class="arena-rule">
<span class="arena-keyword">{if_label}</span>
<select class="arena-condition" aria-label="{condition_label}">{condition_options}</select>
<span class="arena-keyword">{then_label}</span>
<select class="arena-action" aria-label="{action_label}">{action_options}</select>
<button type="button" class="arena-move" data-slot="{slot}" data-move="up" aria-label="{up}"{up_disabled}>↑</button>
<button type="button" class="arena-move" data-slot="{slot}" data-move="down" aria-label="{down}"{down_disabled}>↓</button>
</li>"#,
                    if_label = self.labels.get("if"),
                    then_label = self.labels.get("then"),
                    condition_label = self.labels.get("condition"),
                    action_label = self.labels.get("action"),
                    up = self.labels.get("up"),
                    down = self.labels.get("down"),
                    up_disabled = if slot == 0 { " disabled" } else { "" },
                    down_disabled = if slot == last { " disabled" } else { "" },
                )
            })
            .collect();
        self.page.rules.set_inner_html(&rows);
    }

    fn highlight(&mut self, slot: Option<usize>) {
        if slot == self.highlighted {
            return;
        }
        let Ok(rows) = self.page.rules.query_selector_all("li") else {
            return;
        };
        for (index, active) in [(self.highlighted, false), (slot, true)] {
            if let Some(row) = index.and_then(|index| rows.item(index as u32)?.dyn_into::<Element>().ok()) {
                let _ = row.class_list().toggle_with_force("active", active);
            }
        }
        self.highlighted = slot;
    }

    fn describe(&self, loadout: &Loadout) -> String {
        let range = match loadout.weapon {
            Weapon::Blade => self.labels.get("range.melee"),
            weapon => self.labels.fill("range", &weapon.stats().range.to_string()),
        };
        let mut parts = vec![format!("{} ({range})", self.labels.get(&format!("weapon.{}", loadout.weapon.id())))];
        if loadout.shield {
            parts.push(self.labels.get("module.shield"));
        }
        if loadout.thruster {
            parts.push(self.labels.get("module.thruster"));
        }
        if loadout.armor > 0 {
            parts.push(self.labels.fill("module.armor", &loadout.armor.to_string()));
        }
        parts.join(", ")
    }

    fn bonus_label(&self, bonus: Bonus) -> String {
        match bonus {
            Bonus::Weapon(weapon) => self.labels.get(&format!("bonus.weapon.{}", weapon.id())),
            Bonus::Shield => self.labels.get("bonus.shield"),
            Bonus::Thruster => self.labels.get("bonus.thruster"),
            Bonus::Armor => self.labels.get("bonus.armor"),
            Bonus::RuleSlot => self.labels.get("bonus.rule_slot"),
            Bonus::Condition(condition) => {
                self.labels.fill("bonus.condition", &self.labels.get(&format!("condition.{}", condition.id())))
            }
        }
    }
}

fn element(document: &Document, id: &str) -> Result<Element, JsValue> {
    document.get_element_by_id(id).ok_or_else(|| format!("#{id} absent de la page").into())
}

fn on_click(target: &Element, handler: impl FnMut(web_sys::Event) + 'static) -> Result<(), JsValue> {
    let closure = Closure::<dyn FnMut(web_sys::Event)>::new(handler);
    target.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}

/// Élément cliqué ou son ancêtre le plus proche qui correspond au sélecteur.
fn closest(event: &web_sys::Event, selector: &str) -> Option<Element> {
    event.target()?.dyn_into::<Element>().ok()?.closest(selector).ok().flatten()
}
