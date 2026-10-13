//! Programme d'un robot : règles « SI condition ALORS action ». La première règle dont la condition
//! est vraie et l'action possible l'emporte : une arme ou un bouclier en recharge passe la main à la suivante.

/// Seuil de « PV bas », en part des PV max.
pub const LOW_HEALTH: f32 = 0.5;
/// Distance (centre à centre) sous laquelle l'ennemi est « trop proche ».
pub const CLOSE_DISTANCE: f32 = 90.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Always,
    EnemyInRange,
    LowHealth,
    EnemyLowHealth,
    EnemyShielded,
    EnemyClose,
}

impl Condition {
    /// Débloquées dès le départ ; les autres se gagnent en bonus.
    pub const STARTER: [Condition; 4] =
        [Condition::Always, Condition::EnemyInRange, Condition::LowHealth, Condition::EnemyLowHealth];
    pub const UNLOCKABLE: [Condition; 2] = [Condition::EnemyShielded, Condition::EnemyClose];
    pub const ALL: [Condition; 6] = [
        Condition::Always,
        Condition::EnemyInRange,
        Condition::LowHealth,
        Condition::EnemyLowHealth,
        Condition::EnemyShielded,
        Condition::EnemyClose,
    ];

    /// Identifiant stable : valeur des `<select>` de l'atelier et clé des libellés traduits.
    pub fn id(self) -> &'static str {
        match self {
            Condition::Always => "always",
            Condition::EnemyInRange => "enemy_in_range",
            Condition::LowHealth => "low_health",
            Condition::EnemyLowHealth => "enemy_low_health",
            Condition::EnemyShielded => "enemy_shielded",
            Condition::EnemyClose => "enemy_close",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|condition| condition.id() == id)
    }

    pub fn holds(self, perception: &Perception) -> bool {
        match self {
            Condition::Always => true,
            Condition::EnemyInRange => perception.enemy_distance <= perception.weapon_range,
            Condition::LowHealth => perception.health_ratio < LOW_HEALTH,
            Condition::EnemyLowHealth => perception.enemy_health_ratio < LOW_HEALTH,
            Condition::EnemyShielded => perception.enemy_shielded,
            Condition::EnemyClose => perception.enemy_distance < CLOSE_DISTANCE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Advance,
    Retreat,
    Fire,
    Circle,
    /// Disponible seulement avec le module bouclier.
    Shield,
}

impl Action {
    pub const STARTER: [Action; 4] = [Action::Advance, Action::Retreat, Action::Fire, Action::Circle];
    pub const ALL: [Action; 5] = [Action::Advance, Action::Retreat, Action::Fire, Action::Circle, Action::Shield];

    pub fn id(self) -> &'static str {
        match self {
            Action::Advance => "advance",
            Action::Retreat => "retreat",
            Action::Fire => "fire",
            Action::Circle => "circle",
            Action::Shield => "shield",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.id() == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    pub condition: Condition,
    pub action: Action,
}

/// Ce qu'un robot sait de la situation au moment de décider.
pub struct Perception {
    pub enemy_distance: f32,
    pub weapon_range: f32,
    pub health_ratio: f32,
    pub enemy_health_ratio: f32,
    pub enemy_shielded: bool,
    pub weapon_ready: bool,
    /// Module bouclier présent et rechargé.
    pub shield_ready: bool,
}

impl Action {
    fn possible(self, perception: &Perception) -> bool {
        match self {
            Action::Fire => perception.weapon_ready && perception.enemy_distance <= perception.weapon_range,
            Action::Shield => perception.shield_ready,
            Action::Advance | Action::Retreat | Action::Circle => true,
        }
    }
}

/// Indice de la première règle applicable, aucun sinon.
pub fn decide(rules: &[Rule], perception: &Perception) -> Option<usize> {
    rules.iter().position(|rule| rule.condition.holds(perception) && rule.action.possible(perception))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perception(enemy_distance: f32, health_ratio: f32) -> Perception {
        Perception {
            enemy_distance,
            weapon_range: 200.0,
            health_ratio,
            enemy_health_ratio: 1.0,
            enemy_shielded: false,
            weapon_ready: true,
            shield_ready: false,
        }
    }

    #[test]
    fn first_true_rule_wins() {
        let rules = [
            Rule { condition: Condition::LowHealth, action: Action::Retreat },
            Rule { condition: Condition::EnemyInRange, action: Action::Fire },
            Rule { condition: Condition::Always, action: Action::Advance },
        ];
        assert_eq!(decide(&rules, &perception(100.0, 0.3)), Some(0));
        assert_eq!(decide(&rules, &perception(100.0, 0.9)), Some(1));
        assert_eq!(decide(&rules, &perception(500.0, 0.9)), Some(2));
    }

    #[test]
    fn impossible_action_falls_through() {
        let rules = [
            Rule { condition: Condition::LowHealth, action: Action::Shield },
            Rule { condition: Condition::EnemyInRange, action: Action::Fire },
            Rule { condition: Condition::Always, action: Action::Retreat },
        ];
        let reloading = Perception { weapon_ready: false, ..perception(100.0, 0.3) };
        assert_eq!(decide(&rules, &reloading), Some(2));
    }

    #[test]
    fn ids_round_trip() {
        for condition in Condition::ALL {
            assert_eq!(Condition::from_id(condition.id()), Some(condition));
        }
        for action in Action::ALL {
            assert_eq!(Action::from_id(action.id()), Some(action));
        }
    }

    #[test]
    fn no_matching_rule_means_no_action() {
        let rules = [Rule { condition: Condition::EnemyInRange, action: Action::Fire }];
        assert_eq!(decide(&rules, &perception(500.0, 1.0)), None);
    }
}
