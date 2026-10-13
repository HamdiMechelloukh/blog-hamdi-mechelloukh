//! Équipement d'un robot et bonus gagnés à la fin des combats.

use crate::program::{Action, Condition};
use crate::rng::Rng;
use crate::sim::ROBOT_RADIUS;

const BASE_HEALTH: f32 = 100.0;
const ARMOR_HEALTH: f32 = 25.0;
const BASE_SPEED: f32 = 110.0;
const THRUSTER_SPEED: f32 = 160.0;
const STARTER_RULE_SLOTS: usize = 3;
const MAX_RULE_SLOTS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weapon {
    Cannon,
    Laser,
    Blade,
    Missiles,
}

pub struct WeaponStats {
    pub range: f32,
    pub damage: f32,
    /// Secondes entre deux tirs.
    pub cooldown: f32,
    /// None : arme de mêlée, le coup touche immédiatement.
    pub projectile_speed: Option<f32>,
    pub homing: bool,
}

impl Weapon {
    pub const ALL: [Weapon; 4] = [Weapon::Cannon, Weapon::Laser, Weapon::Blade, Weapon::Missiles];

    /// Clé des libellés traduits.
    pub fn id(self) -> &'static str {
        match self {
            Weapon::Cannon => "cannon",
            Weapon::Laser => "laser",
            Weapon::Blade => "blade",
            Weapon::Missiles => "missiles",
        }
    }

    pub fn stats(self) -> WeaponStats {
        match self {
            Weapon::Cannon => {
                WeaponStats { range: 260.0, damage: 9.0, cooldown: 0.7, projectile_speed: Some(420.0), homing: false }
            }
            Weapon::Laser => {
                WeaponStats { range: 420.0, damage: 6.0, cooldown: 0.5, projectile_speed: Some(900.0), homing: false }
            }
            Weapon::Blade => WeaponStats {
                range: 2.0 * ROBOT_RADIUS + 20.0,
                damage: 13.0,
                cooldown: 0.8,
                projectile_speed: None,
                homing: false,
            },
            Weapon::Missiles => {
                WeaponStats { range: 340.0, damage: 16.0, cooldown: 1.4, projectile_speed: Some(220.0), homing: true }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bonus {
    Weapon(Weapon),
    Shield,
    Thruster,
    Armor,
    RuleSlot,
    Condition(Condition),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loadout {
    pub weapon: Weapon,
    pub shield: bool,
    pub thruster: bool,
    pub armor: u32,
    pub rule_slots: usize,
    pub conditions: Vec<Condition>,
}

impl Loadout {
    pub fn starter() -> Self {
        Self {
            weapon: Weapon::Cannon,
            shield: false,
            thruster: false,
            armor: 0,
            rule_slots: STARTER_RULE_SLOTS,
            conditions: Condition::STARTER.to_vec(),
        }
    }

    pub fn max_health(&self) -> f32 {
        BASE_HEALTH + ARMOR_HEALTH * self.armor as f32
    }

    pub fn speed(&self) -> f32 {
        if self.thruster { THRUSTER_SPEED } else { BASE_SPEED }
    }

    pub fn actions(&self) -> Vec<Action> {
        let mut actions = Action::STARTER.to_vec();
        if self.shield {
            actions.push(Action::Shield);
        }
        actions
    }

    pub fn apply(&mut self, bonus: Bonus) {
        match bonus {
            Bonus::Weapon(weapon) => self.weapon = weapon,
            Bonus::Shield => self.shield = true,
            Bonus::Thruster => self.thruster = true,
            Bonus::Armor => self.armor += 1,
            Bonus::RuleSlot => self.rule_slots += 1,
            Bonus::Condition(condition) => self.conditions.push(condition),
        }
    }

    /// Bonus qui changent encore quelque chose : jamais moins de trois (armes et blindage).
    pub fn available_bonuses(&self) -> Vec<Bonus> {
        let mut bonuses: Vec<Bonus> =
            Weapon::ALL.into_iter().filter(|&weapon| weapon != self.weapon).map(Bonus::Weapon).collect();
        if !self.shield {
            bonuses.push(Bonus::Shield);
        }
        if !self.thruster {
            bonuses.push(Bonus::Thruster);
        }
        bonuses.push(Bonus::Armor);
        if self.rule_slots < MAX_RULE_SLOTS {
            bonuses.push(Bonus::RuleSlot);
        }
        bonuses.extend(
            Condition::UNLOCKABLE
                .into_iter()
                .filter(|condition| !self.conditions.contains(condition))
                .map(Bonus::Condition),
        );
        bonuses
    }

    /// Deux bonus distincts tirés parmi ceux disponibles.
    pub fn offer(&self, rng: &mut Rng) -> [Bonus; 2] {
        let mut bonuses = self.available_bonuses();
        let first = bonuses.swap_remove(rng.below(bonuses.len()));
        let second = bonuses.swap_remove(rng.below(bonuses.len()));
        [first, second]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bonuses_change_the_loadout() {
        let mut loadout = Loadout::starter();
        loadout.apply(Bonus::Armor);
        loadout.apply(Bonus::RuleSlot);
        loadout.apply(Bonus::Shield);
        loadout.apply(Bonus::Condition(Condition::EnemyClose));
        assert_eq!(loadout.max_health(), BASE_HEALTH + ARMOR_HEALTH);
        assert_eq!(loadout.rule_slots, STARTER_RULE_SLOTS + 1);
        assert!(loadout.actions().contains(&Action::Shield));
        assert!(loadout.conditions.contains(&Condition::EnemyClose));
    }

    #[test]
    fn offer_is_two_distinct_available_bonuses() {
        let mut rng = Rng::new(7);
        let mut loadout = Loadout::starter();
        for _ in 0..30 {
            let [first, second] = loadout.offer(&mut rng);
            let available = loadout.available_bonuses();
            assert_ne!(first, second);
            assert!(available.contains(&first) && available.contains(&second));
            loadout.apply(first);
        }
    }
}
