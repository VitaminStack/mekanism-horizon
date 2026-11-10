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
//! - item to chemical in a tank (`oxidizing`): the same, read as a `Recipe` whose
//!   `chemical_output` is the tank id (an `output` with an `amount` is a chemical).
//! - two chemicals to chemical (`chemical_infusing`): `left_input`, `right_input`
//!   {`ingredient`, `amount`}, `output` {`id`, `amount`}, read as an `Infusion`.
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
    infusion: Table<Infusion>,
}

impl Recipes {
    /// The recipes of a processing machine's type.
    pub fn processing(&mut self, host: &dyn Host, kind: &'static str) -> &[Recipe] {
        self.processing.entry(kind).or_default().refresh(host, kind, Recipe::parse)
    }

    /// The Chemical Infuser's recipes (`kind` is its recipe type).
    pub fn infusing(&mut self, host: &dyn Host, kind: &'static str) -> &[Infusion] {
        self.infusion.refresh(host, kind, Infusion::parse)
    }

    /// The chemical conversions (item to chemical) into any buffer.
    pub fn conversion(&mut self, host: &dyn Host) -> &[Conversion] {
        self.conversion.refresh(host, infuser::CONVERSION, Conversion::parse)
    }
}
