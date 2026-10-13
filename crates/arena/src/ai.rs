//! Robot adverse : autant de bonus que le joueur a de victoires, et un style de combat
//! d'autant plus retors que la partie avance.

use crate::loadout::{Loadout, Weapon};
use crate::program::{Action, Condition, Rule};
use crate::rng::Rng;

const fn rule(condition: Condition, action: Action) -> Rule {
    Rule { condition, action }
}

/// Styles de combat, du plus naïf au plus retors : le N-ième n'apparaît qu'après N-1 victoires.
const STYLES: [&[Rule]; 3] = [
    // Fonce en tirant.
    &[rule(Condition::EnemyInRange, Action::Fire), rule(Condition::Always, Action::Advance)],
    // Tire en reculant, ne s'approche qu'hors de portée.
    &[
        rule(Condition::EnemyInRange, Action::Fire),
        rule(Condition::EnemyInRange, Action::Retreat),
        rule(Condition::Always, Action::Advance),
    ],
    // Tourne autour de sa cible pour esquiver entre deux tirs.
    &[
        rule(Condition::EnemyInRange, Action::Fire),
        rule(Condition::EnemyInRange, Action::Circle),
        rule(Condition::Always, Action::Advance),
    ],
];

/// Tactiques placées devant le style dans les emplacements libres, si l'équipement les permet.
const TACTICS: [Rule; 3] = [
    rule(Condition::LowHealth, Action::Shield),
    rule(Condition::EnemyShielded, Action::Circle),
    rule(Condition::EnemyClose, Action::Retreat),
];

pub fn opponent(wins: u32, rng: &mut Rng) -> (Loadout, Vec<Rule>) {
    let mut loadout = Loadout::starter();
    for _ in 0..wins {
        let bonuses = loadout.available_bonuses();
        loadout.apply(bonuses[rng.below(bonuses.len())]);
    }
    let rules = program(&loadout, wins, rng);
    (loadout, rules)
}

fn program(loadout: &Loadout, wins: u32, rng: &mut Rng) -> Vec<Rule> {
    let style = STYLES[rng.below(STYLES.len().min(wins as usize + 1))];
    // Reculer devant l'ennemi ruinerait une lame.
    let blade = loadout.weapon == Weapon::Blade;
    let style = if blade { STYLES[0] } else { style };
    let actions = loadout.actions();
    let mut tactics: Vec<Rule> = TACTICS
        .into_iter()
        .filter(|rule| loadout.conditions.contains(&rule.condition) && actions.contains(&rule.action))
        .filter(|rule| !(blade && rule.action == Action::Retreat))
        .collect();
    let mut rules = Vec::new();
    while rules.len() + style.len() < loadout.rule_slots && !tactics.is_empty() {
        rules.push(tactics.swap_remove(rng.below(tactics.len())));
    }
    rules.extend_from_slice(style);
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_opponent_is_the_naive_starter() {
        let (loadout, rules) = opponent(0, &mut Rng::new(3));
        assert_eq!(loadout, Loadout::starter());
        assert_eq!(rules, STYLES[0]);
    }

    #[test]
    fn programs_fit_the_loadout() {
        for seed in 0..50 {
            for wins in 0..12 {
                let (loadout, rules) = opponent(wins, &mut Rng::new(seed));
                assert!(rules.len() <= loadout.rule_slots);
                assert!(STYLES.iter().any(|style| rules.ends_with(style)));
                let actions = loadout.actions();
                for rule in &rules {
                    assert!(loadout.conditions.contains(&rule.condition) && actions.contains(&rule.action));
                }
            }
        }
    }
}
