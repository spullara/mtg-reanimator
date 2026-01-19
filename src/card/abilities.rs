use crate::game::state::GameState;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during ability execution
#[derive(Error, Debug)]
pub enum GameError {
    #[error("Invalid ability: {0}")]
    InvalidAbility(String),
    #[error("Ability execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

/// Context information passed to ability execution
#[derive(Debug, Clone)]
pub struct TriggerContext {
    pub source_id: usize,
    pub trigger_type: String,
    pub additional_data: HashMap<String, String>,
}

/// Conditions that trigger abilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerCondition {
    OnEnterBattlefield,
    OnCast,
    OnAttack,
    OnMill { count: u32 },
    OnEndStep,
    OnUpkeep,
    OnCreatureEntersBattlefield,
    Manual,
    OnSelfEntersBattlefield,
    OnChapter { chapter: u32 },
}

/// Core ability trait - all abilities implement this
pub trait Ability: Send + Sync {
    fn name(&self) -> &str;
    fn trigger_condition(&self) -> TriggerCondition;
    fn execute(
        &self,
        state: &mut GameState,
        source_id: usize,
        context: &TriggerContext,
    ) -> Result<(), GameError>;
}

/// Surveil ability - look at top N cards, put any in graveyard
#[derive(Debug, Clone)]
pub struct SurveilAbility {
    pub amount: u32,
}

impl Ability for SurveilAbility {
    fn name(&self) -> &str {
        "Surveil"
    }

    fn trigger_condition(&self) -> TriggerCondition {
        TriggerCondition::OnEnterBattlefield
    }

    fn execute(
        &self,
        _state: &mut GameState,
        _source_id: usize,
        _context: &TriggerContext,
    ) -> Result<(), GameError> {
        // Implementation will be in game logic layer
        Ok(())
    }
}

/// Mill ability - mill cards to graveyard
#[derive(Debug, Clone)]
pub struct MillAbility {
    pub amount: u32,
}

impl Ability for MillAbility {
    fn name(&self) -> &str {
        "Mill"
    }

    fn trigger_condition(&self) -> TriggerCondition {
        TriggerCondition::OnEnterBattlefield
    }

    fn execute(
        &self,
        _state: &mut GameState,
        _source_id: usize,
        _context: &TriggerContext,
    ) -> Result<(), GameError> {
        // Implementation will be in game logic layer
        Ok(())
    }
}

/// Draw then discard ability
#[derive(Debug, Clone)]
pub struct DrawDiscardAbility {
    pub draw: u32,
    pub discard: u32,
}

impl Ability for DrawDiscardAbility {
    fn name(&self) -> &str {
        "DrawDiscard"
    }

    fn trigger_condition(&self) -> TriggerCondition {
        TriggerCondition::OnEnterBattlefield
    }

    fn execute(
        &self,
        _state: &mut GameState,
        _source_id: usize,
        _context: &TriggerContext,
    ) -> Result<(), GameError> {
        // Implementation will be in game logic layer
        Ok(())
    }
}

/// Mass reanimation ability (Bringer of the Last Gift effect)
#[derive(Debug, Clone)]
pub struct MassReanimateAbility;

impl Ability for MassReanimateAbility {
    fn name(&self) -> &str {
        "MassReanimate"
    }

    fn trigger_condition(&self) -> TriggerCondition {
        TriggerCondition::OnEnterBattlefield
    }

    fn execute(
        &self,
        _state: &mut GameState,
        _source_id: usize,
        _context: &TriggerContext,
    ) -> Result<(), GameError> {
        // Implementation will be in game logic layer
        Ok(())
    }
}

/// Terror trigger ability - deal damage when creatures enter
#[derive(Debug, Clone)]
pub struct TerrorTriggerAbility;

impl Ability for TerrorTriggerAbility {
    fn name(&self) -> &str {
        "TerrorTrigger"
    }

    fn trigger_condition(&self) -> TriggerCondition {
        TriggerCondition::OnCreatureEntersBattlefield
    }

    fn execute(
        &self,
        _state: &mut GameState,
        _source_id: usize,
        _context: &TriggerContext,
    ) -> Result<(), GameError> {
        // Implementation will be in game logic layer
        Ok(())
    }
}

/// Mind swap ability - Superior Spider-Man copies creature from graveyard
#[derive(Debug, Clone)]
pub struct MindSwapAbility;

impl Ability for MindSwapAbility {
    fn name(&self) -> &str {
        "MindSwap"
    }

    fn trigger_condition(&self) -> TriggerCondition {
        TriggerCondition::OnEnterBattlefield
    }

    fn execute(
        &self,
        _state: &mut GameState,
        _source_id: usize,
        _context: &TriggerContext,
    ) -> Result<(), GameError> {
        // Implementation will be in game logic layer
        Ok(())
    }
}

/// Impending ability - enters with counters, becomes creature when removed
#[derive(Debug, Clone)]
pub struct ImpendingAbility {
    pub counters: u32,
}

impl Ability for ImpendingAbility {
    fn name(&self) -> &str {
        "Impending"
    }

    fn trigger_condition(&self) -> TriggerCondition {
        TriggerCondition::OnSelfEntersBattlefield
    }

    fn execute(
        &self,
        _state: &mut GameState,
        _source_id: usize,
        _context: &TriggerContext,
    ) -> Result<(), GameError> {
        // Implementation will be in game logic layer
        Ok(())
    }
}

/// Channel ability - discard for effect
#[derive(Debug, Clone)]
pub struct ChannelAbility {
    pub effect: String,
}

impl Ability for ChannelAbility {
    fn name(&self) -> &str {
        "Channel"
    }

    fn trigger_condition(&self) -> TriggerCondition {
        TriggerCondition::Manual
    }

    fn execute(
        &self,
        _state: &mut GameState,
        _source_id: usize,
        _context: &TriggerContext,
    ) -> Result<(), GameError> {
        // Implementation will be in game logic layer
        Ok(())
    }
}

/// Saga chapter ability - resolves at specific chapter
#[derive(Debug, Clone)]
pub struct SagaChapterAbility {
    pub chapter: u32,
    pub effect: String,
}

impl Ability for SagaChapterAbility {
    fn name(&self) -> &str {
        "SagaChapter"
    }

    fn trigger_condition(&self) -> TriggerCondition {
        TriggerCondition::OnChapter { chapter: self.chapter }
    }

    fn execute(
        &self,
        _state: &mut GameState,
        _source_id: usize,
        _context: &TriggerContext,
    ) -> Result<(), GameError> {
        // Implementation will be in game logic layer
        Ok(())
    }
}


/// Registry for looking up abilities by name
pub struct AbilityRegistry {
    abilities: HashMap<String, Arc<dyn Ability>>,
}

impl AbilityRegistry {
    /// Create a new ability registry with all standard abilities
    pub fn new() -> Self {
        let mut registry = AbilityRegistry {
            abilities: HashMap::new(),
        };
        registry.register_standard_abilities();
        registry
    }

    /// Register all standard abilities
    fn register_standard_abilities(&mut self) {
        // Surveil abilities
        self.register("Surveil1", Arc::new(SurveilAbility { amount: 1 }));
        self.register("Surveil2", Arc::new(SurveilAbility { amount: 2 }));
        self.register("Surveil3", Arc::new(SurveilAbility { amount: 3 }));
        self.register("Surveil4", Arc::new(SurveilAbility { amount: 4 }));

        // Mill abilities
        self.register("Mill1", Arc::new(MillAbility { amount: 1 }));
        self.register("Mill2", Arc::new(MillAbility { amount: 2 }));
        self.register("Mill3", Arc::new(MillAbility { amount: 3 }));
        self.register("Mill4", Arc::new(MillAbility { amount: 4 }));

        // Draw/Discard abilities
        self.register(
            "DrawDiscard",
            Arc::new(DrawDiscardAbility { draw: 1, discard: 1 }),
        );

        // Mass reanimation
        self.register("MassReanimate", Arc::new(MassReanimateAbility));

        // Terror trigger
        self.register("TerrorTrigger", Arc::new(TerrorTriggerAbility));

        // Mind swap
        self.register("MindSwap", Arc::new(MindSwapAbility));

        // Impending
        self.register("Impending1", Arc::new(ImpendingAbility { counters: 1 }));
        self.register("Impending2", Arc::new(ImpendingAbility { counters: 2 }));
        self.register("Impending3", Arc::new(ImpendingAbility { counters: 3 }));

        // Channel
        self.register(
            "Channel",
            Arc::new(ChannelAbility {
                effect: "default".to_string(),
            }),
        );

        // Saga chapters
        self.register(
            "SagaChapter1",
            Arc::new(SagaChapterAbility {
                chapter: 1,
                effect: "default".to_string(),
            }),
        );
        self.register(
            "SagaChapter2",
            Arc::new(SagaChapterAbility {
                chapter: 2,
                effect: "default".to_string(),
            }),
        );
        self.register(
            "SagaChapter3",
            Arc::new(SagaChapterAbility {
                chapter: 3,
                effect: "default".to_string(),
            }),
        );
    }

    /// Register an ability in the registry
    pub fn register(&mut self, name: &str, ability: Arc<dyn Ability>) {
        self.abilities.insert(name.to_string(), ability);
    }

    /// Get an ability by name
    pub fn get_ability(&self, name: &str) -> Option<Arc<dyn Ability>> {
        self.abilities.get(name).cloned()
    }

    /// Get all registered ability names
    pub fn ability_names(&self) -> Vec<String> {
        self.abilities.keys().cloned().collect()
    }
}

impl Default for AbilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surveil_ability_creation() {
        let ability = SurveilAbility { amount: 2 };
        assert_eq!(ability.name(), "Surveil");
        assert_eq!(ability.trigger_condition(), TriggerCondition::OnEnterBattlefield);
    }

    #[test]
    fn test_mill_ability_creation() {
        let ability = MillAbility { amount: 4 };
        assert_eq!(ability.name(), "Mill");
        assert_eq!(ability.trigger_condition(), TriggerCondition::OnEnterBattlefield);
    }

    #[test]
    fn test_draw_discard_ability_creation() {
        let ability = DrawDiscardAbility { draw: 2, discard: 1 };
        assert_eq!(ability.name(), "DrawDiscard");
        assert_eq!(ability.trigger_condition(), TriggerCondition::OnEnterBattlefield);
    }

    #[test]
    fn test_mass_reanimate_ability_creation() {
        let ability = MassReanimateAbility;
        assert_eq!(ability.name(), "MassReanimate");
        assert_eq!(ability.trigger_condition(), TriggerCondition::OnEnterBattlefield);
    }

    #[test]
    fn test_terror_trigger_ability_creation() {
        let ability = TerrorTriggerAbility;
        assert_eq!(ability.name(), "TerrorTrigger");
        assert_eq!(
            ability.trigger_condition(),
            TriggerCondition::OnCreatureEntersBattlefield
        );
    }

    #[test]
    fn test_mind_swap_ability_creation() {
        let ability = MindSwapAbility;
        assert_eq!(ability.name(), "MindSwap");
        assert_eq!(ability.trigger_condition(), TriggerCondition::OnEnterBattlefield);
    }

    #[test]
    fn test_impending_ability_creation() {
        let ability = ImpendingAbility { counters: 3 };
        assert_eq!(ability.name(), "Impending");
        assert_eq!(
            ability.trigger_condition(),
            TriggerCondition::OnSelfEntersBattlefield
        );
    }

    #[test]
    fn test_channel_ability_creation() {
        let ability = ChannelAbility {
            effect: "draw".to_string(),
        };
        assert_eq!(ability.name(), "Channel");
        assert_eq!(ability.trigger_condition(), TriggerCondition::Manual);
    }

    #[test]
    fn test_saga_chapter_ability_creation() {
        let ability = SagaChapterAbility {
            chapter: 2,
            effect: "mill".to_string(),
        };
        assert_eq!(ability.name(), "SagaChapter");
        assert_eq!(
            ability.trigger_condition(),
            TriggerCondition::OnChapter { chapter: 2 }
        );
    }

    #[test]
    fn test_ability_registry_creation() {
        let registry = AbilityRegistry::new();
        assert!(!registry.ability_names().is_empty());
    }

    #[test]
    fn test_ability_registry_get_surveil() {
        let registry = AbilityRegistry::new();
        let ability = registry.get_ability("Surveil2");
        assert!(ability.is_some());
        assert_eq!(ability.unwrap().name(), "Surveil");
    }

    #[test]
    fn test_ability_registry_get_mill() {
        let registry = AbilityRegistry::new();
        let ability = registry.get_ability("Mill4");
        assert!(ability.is_some());
        assert_eq!(ability.unwrap().name(), "Mill");
    }

    #[test]
    fn test_ability_registry_get_mass_reanimate() {
        let registry = AbilityRegistry::new();
        let ability = registry.get_ability("MassReanimate");
        assert!(ability.is_some());
        assert_eq!(ability.unwrap().name(), "MassReanimate");
    }

    #[test]
    fn test_ability_registry_get_terror_trigger() {
        let registry = AbilityRegistry::new();
        let ability = registry.get_ability("TerrorTrigger");
        assert!(ability.is_some());
        assert_eq!(ability.unwrap().name(), "TerrorTrigger");
    }

    #[test]
    fn test_ability_registry_get_nonexistent() {
        let registry = AbilityRegistry::new();
        let ability = registry.get_ability("NonExistent");
        assert!(ability.is_none());
    }

    #[test]
    fn test_ability_registry_register_custom() {
        let mut registry = AbilityRegistry::new();
        let custom_ability = Arc::new(SurveilAbility { amount: 5 });
        registry.register("CustomSurveil", custom_ability);
        let retrieved = registry.get_ability("CustomSurveil");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "Surveil");
    }

    #[test]
    fn test_trigger_context_creation() {
        let mut data = HashMap::new();
        data.insert("key".to_string(), "value".to_string());
        let context = TriggerContext {
            source_id: 42,
            trigger_type: "etb".to_string(),
            additional_data: data,
        };
        assert_eq!(context.source_id, 42);
        assert_eq!(context.trigger_type, "etb");
    }

    #[test]
    fn test_game_error_display() {
        let error = GameError::InvalidAbility("test".to_string());
        assert_eq!(error.to_string(), "Invalid ability: test");

        let error = GameError::ExecutionFailed("failed".to_string());
        assert_eq!(error.to_string(), "Ability execution failed: failed");

        let error = GameError::InvalidState("bad state".to_string());
        assert_eq!(error.to_string(), "Invalid state: bad state");
    }
}