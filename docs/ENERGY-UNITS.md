# Energy units and limits

Status: decided, option 1 (1:1, as Mekanism 26.3).

## Current state

- Amounts are Horizon's energy: `i32`, shown as "FE" in all screens.
- Mekanism 26.3 has no joules any more: its energy is FE natively
  (`UnitDisplayUtils` shows only `FORGE_ENERGY`; `GeneralConfig` has no
  `feConversionRate`, commit "Remove a couple dead references to Joules").
  The 1.21.x branch still had joules with `feConversionRate` 2.5. The values
  ported here come from 26.3 and are FE: a Basic Energy Cube holds
  4,000,000, the Crusher uses 8 per tick.
- Every ported value fits `i32` except the Creative Energy Cube
  (`Long.MAX_VALUE` in Mekanism), which is capped at `i32::MAX`.
- Transfers never overflow: `energy::Acceptor::accept` (and `cube::transfer`)
  never return more than the room left in the target (tests in `energy.rs`).

## Options

1. Keep 1:1 (recommended). No migration of saved `energy` tags. Exactly
   Mekanism 26.3's values.
2. Apply a conversion rate (Mekanism 1.21.x's 2.5 J per FE). It would not
   match 26.3, whose values are already FE, and some rates would become
   fractions (Crusher 8 / 2.5 = 3.2 per tick) that i32 cannot hold. Every saved
   `energy`, `horizon_energy` and machine buffer would need a migration, and
   Ultimate cube values times 2.5 still fit `i32` (640,000,000), but rates
   would no longer match Mekanism's numbers.

## Limit

Values above `i32::MAX` need a 64-bit buffer in Horizon's `ModEnergyBuffer`
(an upstream change). Until then only the Creative cube is affected.
