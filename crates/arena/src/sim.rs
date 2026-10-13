//! Combat déterministe à pas fixe : mêmes robots, mêmes programmes, même graine → même combat.

use crate::loadout::{Loadout, Weapon};
use crate::program::{Action, Perception, Rule, decide};
use crate::rng::Rng;
use crate::vec2::Vec2;

pub const ARENA_WIDTH: f32 = 800.0;
pub const ARENA_HEIGHT: f32 = 500.0;
pub const ROBOT_RADIUS: f32 = 18.0;
/// Pas de simulation : 60 Hz, quel que soit le rafraîchissement de l'écran.
pub const STEP: f32 = 1.0 / 60.0;
/// Au-delà (60 s), le robot qui a gardé la plus grande part de ses PV gagne ; à égalité, l'IA.
/// Compté en pas entiers : une somme de `STEP` en f32 dérive et n'atteindrait jamais 60 pile.
pub const MAX_STEPS: u32 = 3_600;
const SHIELD_DURATION: f32 = 1.5;
const SHIELD_COOLDOWN: f32 = 5.0;
/// Part des dégâts absorbée par le bouclier.
const SHIELD_ABSORPTION: f32 = 0.75;
/// Dispersion des tirs (radians, de part et d'autre de l'axe).
const SPREAD: f32 = 0.08;
/// Virage maximal des missiles (radians par seconde).
const MISSILE_TURN_RATE: f32 = 2.0;
/// Décalage vertical maximal des positions de départ : casse la symétrie des duels miroirs.
const SPAWN_JITTER: f32 = 120.0;

pub const PLAYER: usize = 0;
pub const ENEMY: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Won,
    Lost,
}

pub struct Robot {
    pub loadout: Loadout,
    pub rules: Vec<Rule>,
    pub position: Vec2,
    /// Le robot fait toujours face à son ennemi.
    pub heading: f32,
    pub health: f32,
    /// Règle exécutée au dernier pas, pour la surligner dans l'atelier.
    pub active_rule: Option<usize>,
    /// Temps restant de bouclier actif.
    pub shield_time: f32,
    reload: f32,
    shield_reload: f32,
}

impl Robot {
    fn new(loadout: Loadout, rules: Vec<Rule>, position: Vec2) -> Self {
        let health = loadout.max_health();
        Self { loadout, rules, position, heading: 0.0, health, active_rule: None, shield_time: 0.0, reload: 0.0, shield_reload: 0.0 }
    }

    pub fn health_ratio(&self) -> f32 {
        self.health.max(0.0) / self.loadout.max_health()
    }

    /// Applique les dégâts ; renvoie true si le bouclier en a absorbé une partie.
    fn take_damage(&mut self, damage: f32) -> bool {
        let shielded = self.shield_time > 0.0;
        self.health -= if shielded { damage * (1.0 - SHIELD_ABSORPTION) } else { damage };
        shielded
    }
}

pub struct Projectile {
    pub position: Vec2,
    pub velocity: Vec2,
    pub owner: usize,
    pub weapon: Weapon,
}

/// Décision d'un robot et géométrie sur laquelle il l'a prise.
struct Plan {
    action: Option<Action>,
    direction: Vec2,
    distance: f32,
}

/// Ce qui s'est passé pendant le dernier pas, pour les effets visuels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Shot { owner: usize, position: Vec2 },
    Slash { owner: usize, position: Vec2 },
    Hit { position: Vec2, shielded: bool },
    Destroyed { position: Vec2 },
}

pub struct Fight {
    pub robots: [Robot; 2],
    pub projectiles: Vec<Projectile>,
    pub steps: u32,
    pub events: Vec<Event>,
    rng: Rng,
}

impl Fight {
    pub fn new(player: (Loadout, Vec<Rule>), enemy: (Loadout, Vec<Rule>), seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let mut spawn_height = || ARENA_HEIGHT / 2.0 + (rng.unit() * 2.0 - 1.0) * SPAWN_JITTER;
        let robots = [
            Robot::new(player.0, player.1, Vec2::new(ARENA_WIDTH * 0.2, spawn_height())),
            Robot::new(enemy.0, enemy.1, Vec2::new(ARENA_WIDTH * 0.8, spawn_height())),
        ];
        Self { robots, projectiles: Vec::new(), steps: 0, events: Vec::new(), rng }
    }

    /// Résultat pour le joueur, None tant que le combat continue.
    pub fn outcome(&self) -> Option<Outcome> {
        let [player, enemy] = &self.robots;
        if enemy.health <= 0.0 && player.health > 0.0 {
            Some(Outcome::Won)
        } else if player.health <= 0.0 {
            Some(Outcome::Lost)
        } else if self.steps >= MAX_STEPS {
            Some(if player.health_ratio() > enemy.health_ratio() { Outcome::Won } else { Outcome::Lost })
        } else {
            None
        }
    }

    /// Avance d'un pas ; sans effet une fois le combat terminé.
    pub fn step(&mut self) {
        if self.outcome().is_some() {
            return;
        }
        self.events.clear();
        // Les deux robots décident sur le même état : sinon le second verrait déjà le premier bouger
        // (et entrerait à portée un pas plus tôt, ce qui suffit à gagner les duels miroirs).
        let plans = [PLAYER, ENEMY].map(|index| self.plan(index));
        for (index, plan) in plans.into_iter().enumerate() {
            self.act(index, plan);
        }
        self.move_projectiles();
        self.steps += 1;
        if self.outcome().is_some() {
            for robot in &self.robots {
                if robot.health <= 0.0 {
                    self.events.push(Event::Destroyed { position: robot.position });
                }
            }
        }
    }

    fn plan(&mut self, index: usize) -> Plan {
        let [first, second] = &mut self.robots;
        let (robot, enemy) = if index == PLAYER { (first, second) } else { (second, first) };
        robot.reload = (robot.reload - STEP).max(0.0);
        robot.shield_reload = (robot.shield_reload - STEP).max(0.0);
        robot.shield_time = (robot.shield_time - STEP).max(0.0);

        let to_enemy = enemy.position - robot.position;
        let distance = to_enemy.length();
        let direction = to_enemy.normalized();
        let weapon = robot.loadout.weapon.stats();
        let perception = Perception {
            enemy_distance: distance,
            weapon_range: weapon.range,
            health_ratio: robot.health_ratio(),
            enemy_health_ratio: enemy.health_ratio(),
            enemy_shielded: enemy.shield_time > 0.0,
            weapon_ready: robot.reload == 0.0,
            shield_ready: robot.loadout.shield && robot.shield_reload == 0.0,
        };
        robot.active_rule = decide(&robot.rules, &perception);
        robot.heading = direction.angle();
        Plan { action: robot.active_rule.map(|index| robot.rules[index].action), direction, distance }
    }

    fn act(&mut self, index: usize, Plan { action, direction, distance }: Plan) {
        let [first, second] = &mut self.robots;
        let (robot, enemy) = if index == PLAYER { (first, second) } else { (second, first) };
        let weapon = robot.loadout.weapon.stats();
        let movement = match action {
            // S'arrête au contact plutôt que de traverser l'ennemi.
            Some(Action::Advance) if distance > 2.0 * ROBOT_RADIUS + 4.0 => direction,
            Some(Action::Retreat) => -direction,
            Some(Action::Circle) => direction.perpendicular(),
            Some(Action::Fire) => {
                robot.reload = weapon.cooldown;
                match weapon.projectile_speed {
                    None => {
                        let shielded = enemy.take_damage(weapon.damage);
                        self.events.push(Event::Slash { owner: index, position: enemy.position });
                        self.events.push(Event::Hit { position: enemy.position, shielded });
                    }
                    Some(speed) => {
                        let angle = robot.heading + (self.rng.unit() * 2.0 - 1.0) * SPREAD;
                        let muzzle = robot.position + Vec2::from_angle(angle) * ROBOT_RADIUS;
                        self.projectiles.push(Projectile {
                            position: muzzle,
                            velocity: Vec2::from_angle(angle) * speed,
                            owner: index,
                            weapon: robot.loadout.weapon,
                        });
                        self.events.push(Event::Shot { owner: index, position: muzzle });
                    }
                }
                Vec2::ZERO
            }
            Some(Action::Shield) => {
                robot.shield_time = SHIELD_DURATION;
                robot.shield_reload = SHIELD_COOLDOWN;
                Vec2::ZERO
            }
            Some(Action::Advance) | None => Vec2::ZERO,
        };
        let position = robot.position + movement * (robot.loadout.speed() * STEP);
        robot.position = Vec2::new(
            position.x.clamp(ROBOT_RADIUS, ARENA_WIDTH - ROBOT_RADIUS),
            position.y.clamp(ROBOT_RADIUS, ARENA_HEIGHT - ROBOT_RADIUS),
        );
    }

    fn move_projectiles(&mut self) {
        let robots = &mut self.robots;
        let events = &mut self.events;
        self.projectiles.retain_mut(|projectile| {
            let target = &mut robots[1 - projectile.owner];
            let weapon = projectile.weapon.stats();
            if weapon.homing {
                let speed = projectile.velocity.length();
                let current = projectile.velocity.angle();
                let wanted = (target.position - projectile.position).angle();
                let turn = wrap_angle(wanted - current).clamp(-MISSILE_TURN_RATE * STEP, MISSILE_TURN_RATE * STEP);
                projectile.velocity = Vec2::from_angle(current + turn) * speed;
            }
            projectile.position = projectile.position + projectile.velocity * STEP;
            if (target.position - projectile.position).length() < ROBOT_RADIUS {
                let shielded = target.take_damage(weapon.damage);
                events.push(Event::Hit { position: projectile.position, shielded });
                return false;
            }
            let Vec2 { x, y } = projectile.position;
            (0.0..=ARENA_WIDTH).contains(&x) && (0.0..=ARENA_HEIGHT).contains(&y)
        });
    }
}

/// Ramène un angle dans [-π, π].
fn wrap_angle(angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    (angle + PI).rem_euclid(TAU) - PI
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loadout::Bonus;
    use crate::program::Condition;

    fn rule(condition: Condition, action: Action) -> Rule {
        Rule { condition, action }
    }

    fn fighter() -> Vec<Rule> {
        vec![rule(Condition::EnemyInRange, Action::Fire), rule(Condition::Always, Action::Advance)]
    }

    fn run(mut fight: Fight) -> (Outcome, Fight) {
        for _ in 0..MAX_STEPS {
            fight.step();
            if let Some(outcome) = fight.outcome() {
                return (outcome, fight);
            }
        }
        panic!("le combat ne s'est pas terminé dans le temps imparti");
    }

    #[test]
    fn same_seed_same_fight() {
        let duel = |seed| Fight::new((Loadout::starter(), fighter()), (Loadout::starter(), fighter()), seed);
        let (first_outcome, first) = run(duel(42));
        let (second_outcome, second) = run(duel(42));
        assert_eq!(first_outcome, second_outcome);
        assert_eq!(first.steps, second.steps);
        assert_eq!(first.robots[PLAYER].health, second.robots[PLAYER].health);
        assert_eq!(first.robots[ENEMY].health, second.robots[ENEMY].health);
    }

    #[test]
    fn every_weapon_destroys_an_idle_robot() {
        for weapon in Weapon::ALL {
            let mut loadout = Loadout::starter();
            loadout.apply(Bonus::Weapon(weapon));
            let (outcome, fight) = run(Fight::new((loadout, fighter()), (Loadout::starter(), Vec::new()), 1));
            assert_eq!(outcome, Outcome::Won, "{weapon:?}");
            assert!(fight.robots[ENEMY].health <= 0.0, "{weapon:?} gagne au temps au lieu de détruire");
            assert!(fight.events.contains(&Event::Destroyed { position: fight.robots[ENEMY].position }));
        }
    }

    #[test]
    fn time_out_with_equal_health_goes_to_the_ai() {
        let coward = vec![rule(Condition::Always, Action::Retreat)];
        let (outcome, fight) = run(Fight::new((Loadout::starter(), coward), (Loadout::starter(), Vec::new()), 1));
        assert_eq!(outcome, Outcome::Lost);
        assert_eq!(fight.steps, MAX_STEPS);
    }

    #[test]
    fn shield_absorbs_damage() {
        let mut shielded = Loadout::starter();
        shielded.apply(Bonus::Shield);
        // Le bouclier s'allume à l'entrée à portée : le premier obus arrive pendant qu'il est actif.
        let turtle = vec![rule(Condition::EnemyInRange, Action::Shield)];
        let mut fight = Fight::new((Loadout::starter(), fighter()), (shielded, turtle), 1);
        while !fight.events.iter().any(|event| matches!(event, Event::Hit { .. })) {
            fight.step();
        }
        assert!(fight.events.iter().any(|event| matches!(event, Event::Hit { shielded: true, .. })));
        assert!(fight.robots[ENEMY].health > fight.robots[ENEMY].loadout.max_health() - Weapon::Cannon.stats().damage);
    }
}
