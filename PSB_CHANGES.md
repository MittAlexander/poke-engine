# Changes on the `psb` branch

Mechanics fixes to poke-engine relative to upstream `f7b147f` (v0.0.48). All target Gen 9
behaviour as implemented in Pokémon Showdown (`f1040ec`), and were found by differential
testing: the same positions were run in Showdown and in the engine, and their outcome
distributions compared.

Effect measured on Gen 9 Random Battles positions, on a held-out set never used for
fixing: the share of positions whose outcome distribution matches Showdown went from
**91.5% to 97.9%**. The share of positions the comparison can represent at all went
from 84% to 96%.

Every change has a regression test in `tests/test_psb_fixes.rs`. All upstream suites
pass for `gen4`–`gen9`, `terastallization` and `champions`; upstream expectations that
encoded the old behaviour were updated (listed per change).

**Keep this file current: every change on this branch gets an entry.**

## Compatibility

- New state is serialized as **optional trailing fields**, so upstream state strings
  still parse (missing fields default to 0/false/none):
  - `Pokemon`: `switch_in_boost_used` (field 29), `last_berry` (field 30)
  - `VolatileStatusDurations`: `healblock` (6), `throatchop` (7), `disable` (8)
- New instructions: `ToggleSwitchInBoostUsed`, `SetLastBerry`.
- New fields: `StatBoosts.evasion`, `Choice.sheer_force_boosted`.
- New item: `LIGHTCLAY` (the `Items` enum is now at the `repr(u8)` limit of 256).
- Python bindings expose all new fields.
- Search cost: 2–5 hit moves now produce up to 4× the branches; `last_berry` is only
  tracked for Harvest/Cud Chew holders, so other berry instruction lists are unchanged.

## Changes

Format: what was wrong → what it does now. *(Showdown reference.)* Tests. Commit.

### Stat changes
- **Evasion couldn't be changed by moves.** `StatBoosts` had no evasion field, so Defog,
  Double Team, Minimize and Sweet Scent had no evasion effect → they now apply −1, +1,
  +2 and −2. *(moves.ts boosts; Defog `onHit` boost.)*
  Tests: `test_defog_lowers_target_evasion`, `test_double_team_raises_evasion`,
  `test_minimize_raises_evasion_by_two`, `test_sweet_scent_lowers_target_evasion_by_two`.
  Upstream tests updated: three Defog instruction lists. `7d28d56`
- **Sticky Web was treated as a self-inflicted drop** → it is now attributed to the
  opposing side, so Clear Body and similar abilities block it and Defiant/Competitive
  trigger. *(stickyweb `this.boost(..., pokemon.side.foe.active[0])`.)*
  Tests: `test_sticky_web_blocked_by_clear_body`, `test_sticky_web_triggers_competitive`.
  `faed740`

### Moves
- **Defog removed the user's own screens** → screens, Safeguard and Mist are cleared only
  on the target's side; hazards still from both sides. *(Defog `removeTarget` vs
  `removeAll`.)* Test: `test_defog_keeps_users_own_screens`, `test_defog_still_clears_hazards`.
  `7d28d56`
- **Curse always cursed the user, whatever its type** → non-Ghost users only get
  +1 Atk/+1 Def/−1 Spe; Ghost users (incl. Tera Ghost) lose ½ max HP and curse the
  target, failing if it is already cursed.
  Tests: `test_non_ghost_curse_only_changes_stats`, `test_ghost_curse_costs_half_hp_and_curses_target`.
  Upstream tests updated: four Sleep Talk tests that pick Curse. `7d28d56`
- **Rest succeeded at full HP** → it fails at full HP, while already asleep, with
  Insomnia/Vital Spirit/Sweet Veil/Comatose/Purifying Salt, with Shields Down above ½ HP,
  with Leaf Guard in sun, and in Electric/Misty Terrain when grounded. (Upstream ignored
  groundedness for Electric Terrain.) It still cures a non-sleep status.
  Tests: `test_rest_fails_at_full_hp`, `test_rest_heals_and_cures_other_status`,
  `test_rest_fails_with_insomnia`. `9f3b292`
- **Spirit Shackle and Anchor Shot didn't trap** → they apply `TRAPPED`.
  Tests: `test_spirit_shackle_traps_target`, `test_anchor_shot_traps_target`. `ed2f06b`
- **2–5 hit moves always hit 3 times** → the engine branches on the hit count:
  2/3 hits 35% each, 4/5 hits 15% each; Loaded Dice 4 or 5 (50/50); Skill Link 5.
  Tests: `test_two_to_five_hit_distribution`, `test_loaded_dice_hits_four_or_five`.
  Upstream test rewritten: `test_scaleshot_only_boosts_once` (checks one boost per use
  across hit-count branches). `f5c63f3`
- **Disable was cosmetic** (the volatile disabled nothing and never ended) → it disables
  the target's last used move (or, for Cursed Body, the attacker's move) for 5
  end-of-turns, 6 if the target already moved this turn, then re-enables it. Only that
  move is re-enabled, so Choice lock is preserved. *(disable condition, `duration++` when
  `!willMove`.)* Tests: `test_disable_expires_and_reenables_the_move`,
  `test_disable_counter_survives_serialization`. `d56c09c`

### Abilities
- **Intrepid Sword / Dauntless Shield triggered on every switch-in** → in Gen 9 and
  Champions they trigger once per battle (`swordBoost` / `shieldBoost`).
  Tests: `test_dauntless_shield_boosts_on_first_switch_in`, `test_dauntless_shield_does_not_boost_twice`,
  `test_intrepid_sword_does_not_boost_twice`, `test_switch_in_boost_flag_survives_serialization`.
  Upstream tests updated: `test_dauntlessshield`, `test_intrepidsword` (gen-aware). `7d28d56`
- **Speed Boost triggered at the end of the turn its holder switched in** → it waits a
  full turn (`activeTurns`). Last-used-move tracking is now enabled when a Speed Boost
  user is present. Tests: `test_speed_boost_not_on_switch_in_turn`,
  `test_speed_boost_after_a_full_turn`. `9f3b292`
- **Harvest was not implemented** → 50% at the end of the turn (100% in sun) to regrow
  the last *eaten* berry (not one that was knocked off); a regrown Sitrus Berry at ≤ ½ HP
  is eaten again immediately. This needed `Pokemon.last_berry` and a branching chance
  pass after the end-of-turn phase.
  Tests: `test_harvest_regrows_berry_half_the_time`, `test_harvested_sitrus_is_eaten_again_at_half_hp`,
  `test_no_harvest_without_an_eaten_berry`, `test_eating_sitrus_records_last_berry`. `ed2f06b`
- **Unburden never activated** (the volatile was checked but never applied) → it
  activates when the holder loses its item during the turn, not when it entered without
  one, and only boosts Speed while itemless.
  Tests: `test_unburden_activates_when_sitrus_is_eaten`, `test_unburden_not_active_when_entering_without_item`.
  `6aa25b6`
- **Sheer Force kept Life Orb recoil** → no recoil when Sheer Force removed the move's
  secondary effects. *(`hasSheerForce` skips AfterMoveSecondarySelf.)*
  Tests: `test_sheer_force_life_orb_has_no_recoil_when_boosted`,
  `test_sheer_force_life_orb_recoil_without_secondaries`. `6aa25b6`
- **Slush Rush and Ice Body only worked in Hail** → they also work in Snow (Gen 9).
  Test: `test_ice_body_heals_in_snow`. `6aa25b6`, `8de81bc`
- **Electromorphosis and Wind Power were not implemented** → being hit (Wind Power: by a
  wind move) charges the holder. Test: `test_electromorphosis_charges_when_hit`. `faed740`
- **Cursed Body was not implemented** → 30% to disable the move that hit it.
  Test: `test_cursed_body_disables_the_move_30_percent`. `d56c09c`

### Items
- **Sitrus Berry was only checked before the holder moved (≤ ¼) and at the end of the
  turn** → it is eaten right after any action that takes the holder to ≤ ½ HP
  (Showdown's per-action Update), including hazard damage on switch-in; Unnerve is
  respected. Tests: `test_sitrus_eaten_right_after_hazard_damage_on_switch_in`,
  `test_sitrus_eaten_mid_turn_before_end_of_turn`.
  Upstream test updated: `test_sitrus_berry_activate_after_taking_damage_when_faster`
  (its holder starts at exactly ½ HP, so it now eats the berry right after its own move).
  `9f3b292`
- **Light Clay was missing** → screens set by its holder last 8 turns.
  Test: `test_light_clay_extends_reflect_to_eight_turns`. `e20f124`

### Volatiles and timers
- **Heal Block (Psychic Noise) never expired** → it lasts two turns.
  Tests: `test_psychic_noise_heal_block_lasts_two_turns`, `test_heal_block_timer_survives_serialization`.
  Upstream tests updated: three Heal Block tests (end-of-turn timer instruction). `9f3b292`
- **Heal Block only blocked healing moves** → it blocks every heal: Leftovers, Black
  Sludge, Sitrus (not eaten), Regenerator, Ice Body, Poison Heal, Rain Dish, Dry Skin,
  Grassy Terrain, Wish, drain, Leech Seed's heal for the seeder, and positive move heals.
  HP costs (Curse, Belly Drum) still apply. *(healblock `onTryHeal`, Sitrus `onTryEatItem`.)*
  Tests: `test_heal_block_stops_leftovers`, `test_heal_block_stops_sitrus_from_being_eaten`,
  `test_heal_block_stops_drain`. `8de81bc`
- **Throat Chop never expired** → it lasts two turns (shared two-turn counter with
  Heal Block). Test: `test_throat_chop_lasts_two_turns`. `6aa25b6`
- **Charge never ended**, so every later Electric move was doubled → it ends once its
  holder uses an Electric move. Test: `test_charge_ends_after_an_electric_move`. `faed740`

### Turn order
- **Double switches always ran side one first** → the faster outgoing Pokémon switches
  first (reversed in Trick Room; ties go to side one, which Showdown randomises), so
  switch-in effects such as Intimidate hit the right Pokémon.
  Tests: `test_double_switch_faster_side_switches_first`, `test_double_switch_slower_intimidate_hits_incoming`.
  `ed2f06b`
- **End-of-turn effects ran status damage first** → Showdown's residual order:
  weather abilities, Leftovers/Black Sludge (5.4), Shed Skin/Hydration (5.3), then Leech
  Seed (8), poison (9), burn (10), then late effects (Speed Boost, orbs: 28). E.g. a
  burned Leftovers holder at 330/352 heals to full and is burned back to 330 instead of
  ending at full. Test: `test_leftovers_before_burn_damage`.
  Upstream tests updated: `test_hydration_end_of_turn`, `test_shedskin_end_of_turn`
  (cured before poison damage). `622268e`

## Known remaining differences

Not addressed on this branch; see the backlog in the PokeSymbolBot repository.

- Approximations by design: average damage roll with one kill/crit branch per hit;
  Triple Axel / Population Bomb skip per-hit accuracy; Shed Skin always succeeds;
  confusion has no duration; fixed Outrage lock length.
- Unimplemented: Imposter/Transform, Take Heart, Toxic Chain, Poison Puppeteer, Beat Up,
  Beak Blast, Dancer, Lightning Rod / Storm Drain absorption, Soul-Heart, Mirror Armor,
  Cute Charm, Leppa Berry, Scope Lens, Rage Fist's hit counter.
