#![cfg(any(feature = "gen9", feature = "terastallization"))]
//! Regression tests for mechanics fixes found by differential testing against
//! Pokemon Showdown (gen 9). Each test checks the resulting state of every branch
//! rather than the exact instruction list, so it stays valid if the engine
//! reorders or merges instructions.

use poke_engine::choices::Choices;
use poke_engine::engine::generate_instructions::generate_instructions_from_move_pair;
use poke_engine::engine::state::MoveChoice;
use poke_engine::state::{PokemonMoveIndex, State};

/// All branches of a turn as (probability in %, resulting state).
fn outcomes(state: &State, move_one: Choices, move_two: Choices) -> Vec<(f32, State)> {
    let mut state = state.clone();
    state.side_one.get_active().replace_move(PokemonMoveIndex::M0, move_one);
    state.side_two.get_active().replace_move(PokemonMoveIndex::M0, move_two);
    let m = MoveChoice::Move(PokemonMoveIndex::M0);
    let branches = generate_instructions_from_move_pair(&mut state, &m, &m, false);
    branches
        .into_iter()
        .map(|b| {
            let mut s = state.clone();
            s.apply_instructions(&b.instruction_list);
            (b.percentage, s)
        })
        .collect()
}

fn always<F: Fn(&State) -> bool>(branches: &[(f32, State)], f: F) -> bool {
    !branches.is_empty() && branches.iter().all(|(_, s)| f(s))
}

// ------------------------------------------------------------------ evasion boosts

#[test]
fn test_defog_lowers_target_evasion() {
    let state = State::default();
    let b = outcomes(&state, Choices::DEFOG, Choices::SPLASH);
    assert!(always(&b, |s| s.side_two.evasion_boost == -1));
}

#[test]
fn test_defog_still_clears_hazards() {
    let mut state = State::default();
    state.side_one.side_conditions.spikes = 2;
    state.side_two.side_conditions.stealth_rock = 1;
    let b = outcomes(&state, Choices::DEFOG, Choices::SPLASH);
    assert!(always(&b, |s| s.side_one.side_conditions.spikes == 0
        && s.side_two.side_conditions.stealth_rock == 0));
}

#[test]
fn test_double_team_raises_evasion() {
    let b = outcomes(&State::default(), Choices::DOUBLETEAM, Choices::SPLASH);
    assert!(always(&b, |s| s.side_one.evasion_boost == 1));
}

#[test]
fn test_minimize_raises_evasion_by_two() {
    let b = outcomes(&State::default(), Choices::MINIMIZE, Choices::SPLASH);
    assert!(always(&b, |s| s.side_one.evasion_boost == 2));
}

#[test]
fn test_sweet_scent_lowers_target_evasion_by_two() {
    let b = outcomes(&State::default(), Choices::SWEETSCENT, Choices::SPLASH);
    assert!(always(&b, |s| s.side_two.evasion_boost == -2));
}

#[test]
fn test_defog_keeps_users_own_screens() {
    let mut state = State::default();
    state.side_one.side_conditions.reflect = 3;
    state.side_two.side_conditions.light_screen = 3;
    let b = outcomes(&state, Choices::DEFOG, Choices::SPLASH);
    // screens tick down at the end of the turn, so 3 -> 2 rather than removed
    assert!(always(&b, |s| s.side_one.side_conditions.reflect == 2
        && s.side_two.side_conditions.light_screen == 0));
}

// ------------------------------------------------------------------ Curse

use poke_engine::engine::state::PokemonVolatileStatus;
use poke_engine::state::PokemonType;

#[test]
fn test_non_ghost_curse_only_changes_stats() {
    let b = outcomes(&State::default(), Choices::CURSE, Choices::SPLASH);
    assert!(always(&b, |s| s.side_one.attack_boost == 1
        && s.side_one.defense_boost == 1
        && s.side_one.speed_boost == -1
        && !s.side_one.volatile_statuses.contains(&PokemonVolatileStatus::CURSE)
        && !s.side_two.volatile_statuses.contains(&PokemonVolatileStatus::CURSE)));
}

#[test]
fn test_ghost_curse_costs_half_hp_and_curses_target() {
    let mut state = State::default();
    state.side_one.get_active().types = (PokemonType::GHOST, PokemonType::TYPELESS);
    let maxhp = state.side_one.get_active().maxhp;
    let b = outcomes(&state, Choices::CURSE, Choices::SPLASH);
    assert!(always(&b, |s| s.side_one.attack_boost == 0
        && s.side_one.get_active_immutable().hp == maxhp - maxhp / 2
        && s.side_two.volatile_statuses.contains(&PokemonVolatileStatus::CURSE)
        && !s.side_one.volatile_statuses.contains(&PokemonVolatileStatus::CURSE)));
}

// ------------------------------------------------------------------ once-per-battle switch-in boosts

use poke_engine::engine::abilities::Abilities;
use poke_engine::state::PokemonIndex;

/// Side one switches to `to` on a normal turn while side two uses Splash.
fn switch_outcomes(state: &State, to: PokemonIndex) -> Vec<(f32, State)> {
    let mut state = state.clone();
    state.side_two.get_active().replace_move(PokemonMoveIndex::M0, Choices::SPLASH);
    let branches = generate_instructions_from_move_pair(
        &mut state, &MoveChoice::Switch(to), &MoveChoice::Move(PokemonMoveIndex::M0), false);
    branches.into_iter().map(|b| {
        let mut s = state.clone();
        s.apply_instructions(&b.instruction_list);
        (b.percentage, s)
    }).collect()
}

#[test]
fn test_dauntless_shield_boosts_on_first_switch_in() {
    let mut state = State::default();
    state.side_one.pokemon[PokemonIndex::P1].ability = Abilities::DAUNTLESSSHIELD;
    let b = switch_outcomes(&state, PokemonIndex::P1);
    assert!(always(&b, |s| s.side_one.defense_boost == 1
        && s.side_one.pokemon[PokemonIndex::P1].switch_in_boost_used));
}

#[test]
fn test_dauntless_shield_does_not_boost_twice() {
    let mut state = State::default();
    state.side_one.pokemon[PokemonIndex::P1].ability = Abilities::DAUNTLESSSHIELD;
    state.side_one.pokemon[PokemonIndex::P1].switch_in_boost_used = true;
    let b = switch_outcomes(&state, PokemonIndex::P1);
    assert!(always(&b, |s| s.side_one.defense_boost == 0));
}

#[test]
fn test_intrepid_sword_does_not_boost_twice() {
    let mut state = State::default();
    state.side_one.pokemon[PokemonIndex::P1].ability = Abilities::INTREPIDSWORD;
    state.side_one.pokemon[PokemonIndex::P1].switch_in_boost_used = true;
    let b = switch_outcomes(&state, PokemonIndex::P1);
    assert!(always(&b, |s| s.side_one.attack_boost == 0));
}

#[test]
fn test_switch_in_boost_flag_survives_serialization() {
    let mut state = State::default();
    state.side_one.pokemon[PokemonIndex::P1].switch_in_boost_used = true;
    let s2 = State::deserialize(&state.serialize());
    assert!(s2.side_one.pokemon[PokemonIndex::P1].switch_in_boost_used);
}

// ------------------------------------------------------------------ Rest

use poke_engine::state::PokemonStatus;

#[test]
fn test_rest_fails_at_full_hp() {
    let state = State::default();
    let b = outcomes(&state, Choices::REST, Choices::SPLASH);
    assert!(always(&b, |s| s.side_one.get_active_immutable().status == PokemonStatus::NONE));
}

#[test]
fn test_rest_heals_and_cures_other_status() {
    let mut state = State::default();
    state.side_one.get_active().hp = 30;
    state.side_one.get_active().status = PokemonStatus::BURN;
    let b = outcomes(&state, Choices::REST, Choices::SPLASH);
    assert!(always(&b, |s| {
        let a = s.side_one.get_active_immutable();
        a.status == PokemonStatus::SLEEP && a.hp == a.maxhp && a.rest_turns == 3
    }));
}

#[test]
fn test_rest_fails_with_insomnia() {
    let mut state = State::default();
    state.side_one.get_active().hp = 30;
    state.side_one.get_active().ability = Abilities::INSOMNIA;
    let b = outcomes(&state, Choices::REST, Choices::SPLASH);
    assert!(always(&b, |s| s.side_one.get_active_immutable().hp == 30));
}

// ------------------------------------------------------------------ Sitrus Berry timing

use poke_engine::engine::items::Items;

#[test]
fn test_sitrus_eaten_right_after_hazard_damage_on_switch_in() {
    let mut state = State::default();
    state.side_one.side_conditions.stealth_rock = 1;
    let incoming = &mut state.side_one.pokemon[PokemonIndex::P1];
    incoming.item = Items::SITRUSBERRY;
    incoming.hp = incoming.maxhp / 2 + 5; // rocks (1/8) take it below half
    let b = switch_outcomes(&state, PokemonIndex::P1);
    assert!(always(&b, |s| s.side_one.pokemon[PokemonIndex::P1].item == Items::NONE));
}

#[test]
fn test_sitrus_eaten_mid_turn_before_end_of_turn() {
    let mut state = State::default();
    state.side_two.get_active().item = Items::SITRUSBERRY;
    state.side_two.get_active().hp = 60;
    // side one moves first (equal speed is a tie, so make it faster)
    state.side_one.get_active().speed = 200;
    let b = outcomes(&state, Choices::TACKLE, Choices::SPLASH);
    // tackle takes it to <= 50%: the berry is gone in every branch that didn't faint
    assert!(always(&b, |s| s.side_two.get_active_immutable().item == Items::NONE));
}

// ------------------------------------------------------------------ Speed Boost

#[test]
fn test_speed_boost_not_on_switch_in_turn() {
    let mut state = State::default();
    state.side_one.pokemon[PokemonIndex::P1].ability = Abilities::SPEEDBOOST;
    state.set_conditional_mechanics(); // as State::deserialize and the bindings do
    let b = switch_outcomes(&state, PokemonIndex::P1);
    assert!(always(&b, |s| s.side_one.speed_boost == 0));
}

#[test]
fn test_speed_boost_after_a_full_turn() {
    let mut state = State::default();
    state.side_one.get_active().ability = Abilities::SPEEDBOOST;
    let b = outcomes(&state, Choices::SPLASH, Choices::SPLASH);
    assert!(always(&b, |s| s.side_one.speed_boost == 1));
}


// ------------------------------------------------------------------ Heal Block duration

#[test]
fn test_psychic_noise_heal_block_lasts_two_turns() {
    let mut state = State::default();
    state.side_one.get_active().speed = 200;
    // turn 1: applied, survives the first end of turn
    let b = outcomes(&state, Choices::PSYCHICNOISE, Choices::SPLASH);
    assert!(always(&b, |s| s.side_two.volatile_statuses.contains(&PokemonVolatileStatus::HEALBLOCK)));
    // turn 2: removed at the second end of turn
    let after_one = b[0].1.clone();
    let b2 = outcomes(&after_one, Choices::SPLASH, Choices::SPLASH);
    assert!(always(&b2, |s| !s.side_two.volatile_statuses.contains(&PokemonVolatileStatus::HEALBLOCK)
        && s.side_two.volatile_status_durations.healblock == 0));
}

#[test]
fn test_heal_block_timer_survives_serialization() {
    let mut state = State::default();
    state.side_two.volatile_status_durations.healblock = 1;
    assert_eq!(State::deserialize(&state.serialize()).side_two.volatile_status_durations.healblock, 1);
}
