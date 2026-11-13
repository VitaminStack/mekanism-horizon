//! Mekanism's recipe files, shipped by the content mod, parsed once per recipe
//! table: one shape for every processing machine (`Recipe`) and the chemical
//! conversions that fill a machine's chemical buffer (`Conversion`).
//!
//! Shapes read (Mekanism 26.3 serializers):
//! - item to item (`enriching`, `crushing`): `input` {`ingredient`, `count`}, `output`.
//! - item and chemical to item (`metallurgic_infusing`, `compressing`):
//!   `item_input`, `chemical_input` {`ingredient`, `amount`}, `output`, and
//!   `per_tick_usage` (the chemical amount is used every tick, not once).
//! - two items to item (`combining`): `main_input`, `extra_input`, `output`.
//! - item to item and chance item (`sawing`): `input`, `main_output` and/or
//!   `secondary_output` with `secondary_chance`.
//! - item to chemical (`chemical_conversion`): `input`, `output` {`id`, `amount`}.
//! - item to chemical in a tank (`oxidizing`, `pigment_extracting`; `liquification`'s fluid is a
//!   tank id too): the same, read as a `Recipe` whose
//!   `chemical_output` is the tank id (an `output` with an `amount` is a chemical).
//! - two chemicals to chemical (`chemical_infusing`, `pigment_mixing`): `left_input`, `right_input`
//!   {`ingredient`, `amount`}, `output` {`id`, `amount`}, read as an `Infusion`.
//! - fluid to two chemicals (`separating`): `input` {`ingredient`, `amount`},
//!   `output` {`left`, `right`} {`id`, `amount`}, read as a `Separation`. The
//!   importer turns the fluid tag into the fluid's id (`#c:brine` becomes
//!   `mekanism:brine`).
//! - chemical and fluid for each other (`rotary`): `chemical_input`,
//!   `chemical_output`, `fluid_input`, `fluid_output`, read as a `Rotation`.
//! - item and chemical to item (`painting`): `item_input`, `chemical_input` {`ingredient`, `amount`},
//!   `output`; the chemical comes from the machine's input tank and is used once at the end.
//! - item and chemical to chemical (`dissolution`): `item_input`, `chemical_input`
//!   {`ingredient`, `amount`}, `output` {`id`, `amount`}, `per_tick_usage`; a `Recipe`
//!   whose chemical comes from an input tank and goes to the output tank.
//! - fluid and chemical to chemical (`washing`): `fluid_input`, `chemical_input`
//!   {`ingredient`, `amount`}, `output` {`id`, `amount`}, read as a `Washing`.
//! - chemical to item (`crystallizing`): `input` {`ingredient`, `amount`}, `output` {`id`},
//!   read as a `Crystal`.
//!
//! A recipe that names an unknown item is skipped, as Mekanism skips recipes
//! whose ingredients do not resolve. Chemicals stay ids here: each machine
//! matches them against the types its buffer takes (`machine::Buffer`).

use std::collections::HashMap;

use horizon_client::mod_api::serde_json::Value;
use horizon_client::mod_api::{Host, Ingredient, ItemStack};

use super::chemicals;
use super::infuser;

/// A chemical a recipe uses from the machine's buffer.
pub struct ChemicalInput {
    /// Mekanism's id (`mekanism:osmium`).
    pub id: String,
    pub amount: i32,
    /// Used every tick of the operation (`per_tick_usage`) rather than once at its end.
    pub per_tick: bool,
}

/// One operation of a processing machine: `count` of `input` (with `extra`
/// and `chemical`, when set) make `output` and, by chance, `secondary`.
pub struct Recipe {
    pub input: Ingredient,
    pub count: u32,
    pub extra: Option<(Ingredient, u32)>,
    pub chemical: Option<ChemicalInput>,
    /// None for a sawing recipe with just a secondary output, and for one
    /// whose output is a chemical.
    pub output: Option<ItemStack>,
    /// A chemical output as (tank id, mB): put in the machine's output tank.
    pub chemical_output: Option<(String, i32)>,
    /// The output and its chance in (0, 1].
    pub secondary: Option<(ItemStack, f64)>,
}

/// An item ingredient with its count (`{ingredient, count}`), count 1 by default.
fn sized(host: &dyn Host, j: &Value) -> Option<(Ingredient, u32)> {
    let count = j["count"].as_u64().map_or(Some(1), |c| u32::try_from(c).ok().filter(|&c| c > 0))?;
    Some((host.recipe_ingredient(&j["ingredient"])?, count))
}

/// An optional field: absent is `Some(None)`, present but unreadable is `None`.
fn optional<T>(v: &Value, read: impl FnOnce(&Value) -> Option<T>) -> Option<Option<T>> {
    if v.is_null() { Some(None) } else { read(v).map(Some) }
}

impl Recipe {
    fn parse(host: &dyn Host, j: &Value) -> Option<Self> {
        let item = ["item_input", "main_input", "input"].iter().map(|k| &j[*k]).find(|v| v.is_object())?;
        let (input, count) = sized(host, item)?;
        let chemical = optional(&j["chemical_input"], |c| {
            Some(ChemicalInput {
                id: c["ingredient"].as_str()?.to_owned(),
                amount: positive(&c["amount"])?,
                per_tick: j["per_tick_usage"].as_bool().unwrap_or(false),
            })
        })?;
        let main = if j["output"].is_null() { &j["main_output"] } else { &j["output"] };
        let chemical_output = if main["amount"].is_number() { Some(chemical_stack(main)?) } else { None };
        let output = if chemical_output.is_some() { None } else { optional(main, |o| host.recipe_result(o))? };
        let secondary = optional(&j["secondary_output"], |o| {
            let chance = j["secondary_chance"].as_f64().filter(|c| *c > 0.0 && *c <= 1.0)?;
            Some((host.recipe_result(o)?, chance))
        })?;
        if output.is_none() && secondary.is_none() && chemical_output.is_none() {
            return None;
        }
        Some(Recipe {
            input,
            count,
            extra: optional(&j["extra_input"], |e| sized(host, e))?,
            chemical,
            output,
            chemical_output,
            secondary,
        })
    }
}

/// A chemical result `{id, amount}` as (tank id, mB).
fn chemical_stack(j: &Value) -> Option<(String, i32)> {
    Some((chemicals::from_recipe(j["id"].as_str()?)?, positive(&j["amount"])?))
}

/// A chemical ingredient `{ingredient, amount}` as (tank id, mB).
fn chemical_ingredient(j: &Value) -> Option<(String, i32)> {
    Some((chemicals::from_recipe(j["ingredient"].as_str()?)?, positive(&j["amount"])?))
}

/// Two chemicals make a third (`chemical_infusing`). The inputs name no side:
/// the machine takes them from its two tanks in either order.
pub struct Infusion {
    pub left: (String, i32),
    pub right: (String, i32),
    pub output: (String, i32),
}

impl Infusion {
    fn parse(_: &dyn Host, j: &Value) -> Option<Self> {
        Some(Infusion {
            left: chemical_ingredient(&j["left_input"])?,
            right: chemical_ingredient(&j["right_input"])?,
            output: chemical_stack(&j["output"])?,
        })
    }

    /// Whether `id` is one of the inputs, and then the other one: what a tank
    /// may hold beside it.
    pub fn partner(&self, id: &str) -> Option<&str> {
        match (self.left.0 == id, self.right.0 == id) {
            (true, _) => Some(&self.right.0),
            (_, true) => Some(&self.left.0),
            _ => None,
        }
    }
}

/// A fluid and a chemical make a chemical (`washing`): the Chemical Washer's
/// dirty slurry and water make clean slurry.
pub struct Washing {
    /// The fluid's tank id and mB.
    pub fluid: (String, i32),
    pub chemical: (String, i32),
    pub output: (String, i32),
}

impl Washing {
    fn parse(_: &dyn Host, j: &Value) -> Option<Self> {
        Some(Washing {
            fluid: fluid_ingredient(&j["fluid_input"])?,
            chemical: chemical_ingredient(&j["chemical_input"])?,
            output: chemical_stack(&j["output"])?,
        })
    }
}

/// A chemical makes an item (`crystallizing`): `chemical` mB of a slurry make `output`.
pub struct Crystal {
    pub chemical: (String, i32),
    pub output: ItemStack,
}

impl Crystal {
    fn parse(host: &dyn Host, j: &Value) -> Option<Self> {
        Some(Crystal { chemical: chemical_ingredient(&j["input"])?, output: host.recipe_result(&j["output"])? })
    }
}

/// A fluid splits into two chemicals (`separating`).
pub struct Separation {
    /// The fluid's tank id and mB.
    pub input: (String, i32),
    pub left: (String, i32),
    pub right: (String, i32),
}

impl Separation {
    fn parse(_: &dyn Host, j: &Value) -> Option<Self> {
        Some(Separation {
            input: fluid_ingredient(&j["input"])?,
            left: chemical_stack(&j["output"]["left"])?,
            right: chemical_stack(&j["output"]["right"])?,
        })
    }
}

/// A fluid ingredient `{ingredient, amount}` as (tank id, mB); the importer has
/// resolved a fluid tag to its fluid. A chemical is no fluid.
fn fluid_ingredient(j: &Value) -> Option<(String, i32)> {
    let id = j["ingredient"].as_str().filter(|id| !id.starts_with('#'))?;
    Some((id.to_owned(), positive(&j["amount"])?))
}

/// A chemical and a fluid that turn into each other (`rotary`), with the
/// amounts of each direction.
pub struct Rotation {
    /// The chemical's tank id and the fluid's.
    pub chemical: String,
    pub fluid: String,
    /// Chemical to fluid: `chemical_in` mB of the chemical make `fluid_out` of the fluid.
    pub chemical_in: i32,
    pub fluid_out: i32,
    /// Fluid to chemical.
    pub fluid_in: i32,
    pub chemical_out: i32,
}

impl Rotation {
    fn parse(_: &dyn Host, j: &Value) -> Option<Self> {
        let (chemical, chemical_in) = chemical_ingredient(&j["chemical_input"])?;
        let (fluid, fluid_out) = chemical_stack_raw(&j["fluid_output"])?;
        let fluid_in = positive(&j["fluid_input"]["amount"])?;
        let (out, chemical_out) = chemical_stack(&j["chemical_output"])?;
        (out == chemical).then_some(Rotation { chemical, fluid, chemical_in, fluid_out, fluid_in, chemical_out })
    }
}

/// A result `{id, amount}` with the id as it is (a fluid's tank id).
fn chemical_stack_raw(j: &Value) -> Option<(String, i32)> {
    Some((j["id"].as_str()?.to_owned(), positive(&j["amount"])?))
}

/// One item of `input` gives `amount` of chemical `chemical`.
pub struct Conversion {
    pub input: Ingredient,
    pub chemical: String,
    pub amount: i32,
}

impl Conversion {
    fn parse(host: &dyn Host, j: &Value) -> Option<Self> {
        Some(Conversion {
            input: host.recipe_ingredient(&j["input"]["ingredient"])?,
            chemical: j["output"]["id"].as_str()?.to_owned(),
            amount: positive(&j["output"]["amount"])?,
        })
    }
}

/// A positive amount that fits i32.
fn positive(v: &Value) -> Option<i32> {
    v.as_i64().and_then(|a| i32::try_from(a).ok()).filter(|&a| a > 0)
}

/// The parsed recipes of one type, kept while the server's table for it is
/// the same one (its slice's address and length); `/reload` gives new ones.
struct Table<T> {
    key: [usize; 2],
    recipes: Vec<T>,
}

impl<T> Default for Table<T> {
    fn default() -> Self {
        Table { key: [0; 2], recipes: Vec::new() }
    }
}

impl<T> Table<T> {
    fn refresh(&mut self, host: &dyn Host, kind: &str, parse: fn(&dyn Host, &Value) -> Option<T>) -> &[T] {
        let all = host.recipes_of_type(kind);
        let key = [all.as_ptr() as usize, all.len()];
        if key != self.key {
            self.key = key;
            self.recipes = all.iter().filter_map(|r| parse(host, &r.json)).collect();
        }
        &self.recipes
    }
}

#[derive(Default)]
pub struct Recipes {
    processing: HashMap<&'static str, Table<Recipe>>,
    conversion: Table<Conversion>,
    /// By recipe type: the Chemical Infuser and the Pigment Mixer share the shape.
    infusion: HashMap<&'static str, Table<Infusion>>,
    separation: Table<Separation>,
    rotation: Table<Rotation>,
    washing: Table<Washing>,
    crystal: Table<Crystal>,
}

impl Recipes {
    /// The recipes of a processing machine's type.
    pub fn processing(&mut self, host: &dyn Host, kind: &'static str) -> &[Recipe] {
        self.processing.entry(kind).or_default().refresh(host, kind, Recipe::parse)
    }

    /// The Chemical Infuser's and Pigment Mixer's recipes (`kind` is the recipe type).
    pub fn infusing(&mut self, host: &dyn Host, kind: &'static str) -> &[Infusion] {
        self.infusion.entry(kind).or_default().refresh(host, kind, Infusion::parse)
    }

    /// The Electrolytic Separator's recipes.
    pub fn separating(&mut self, host: &dyn Host, kind: &'static str) -> &[Separation] {
        self.separation.refresh(host, kind, Separation::parse)
    }

    /// The Rotary Condensentrator's recipes.
    pub fn rotating(&mut self, host: &dyn Host, kind: &'static str) -> &[Rotation] {
        self.rotation.refresh(host, kind, Rotation::parse)
    }

    /// The Chemical Washer's recipes.
    pub fn washing(&mut self, host: &dyn Host, kind: &'static str) -> &[Washing] {
        self.washing.refresh(host, kind, Washing::parse)
    }

    /// The Chemical Crystallizer's recipes.
    pub fn crystallizing(&mut self, host: &dyn Host, kind: &'static str) -> &[Crystal] {
        self.crystal.refresh(host, kind, Crystal::parse)
    }

    /// The chemical conversions (item to chemical) into any buffer.
    pub fn conversion(&mut self, host: &dyn Host) -> &[Conversion] {
        self.conversion.refresh(host, infuser::CONVERSION, Conversion::parse)
    }
}
