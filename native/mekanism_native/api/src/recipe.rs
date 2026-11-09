//! Mekanism's recipe files, shipped by the content mod, parsed once per recipe
//! table: one shape for every processing machine (`Recipe`) and the chemical
//! conversions that fill an infuse buffer (`Conversion`).
//!
//! Shapes read (Mekanism 26.3 serializers):
//! - item to item (`enriching`, `crushing`): `input` {`ingredient`, `count`}, `output`.
//! - item and chemical to item (`metallurgic_infusing`): `item_input`,
//!   `chemical_input` {`ingredient`, `amount`}, `output`.
//! - item to chemical (`chemical_conversion`): `input`, `output` {`id`, `amount`}.
//!
//! A recipe that names something not ported yet (a chemical other than the
//! infuse types, an unknown item) is skipped, as Mekanism skips recipes whose
//! ingredients do not resolve.

use std::collections::HashMap;

use horizon_client::mod_api::serde_json::Value;
use horizon_client::mod_api::{Host, Ingredient, ItemStack};

use super::infuser;

/// One operation of a processing machine: `count` of `input` (and `chemical`
/// from the infuse buffer, when set) make `output`.
pub struct Recipe {
    pub input: Ingredient,
    pub count: u32,
    /// (`infuser::TYPES` index, amount) used per operation.
    pub chemical: Option<(usize, i32)>,
    pub output: ItemStack,
}

impl Recipe {
    fn parse(host: &dyn Host, j: &Value) -> Option<Self> {
        let item = if j["item_input"].is_object() { &j["item_input"] } else { &j["input"] };
        let chemical = match &j["chemical_input"] {
            Value::Null => None,
            c => Some((infuser::type_index(c["ingredient"].as_str()?)?, positive(&c["amount"])?)),
        };
        Some(Recipe {
            input: host.recipe_ingredient(&item["ingredient"])?,
            count: item["count"].as_u64().map_or(Some(1), |c| u32::try_from(c).ok().filter(|&c| c > 0))?,
            chemical,
            output: host.recipe_result(&j["output"])?,
        })
    }
}

/// One item of `input` gives `amount` of infuse type `kind`.
pub struct Conversion {
    pub input: Ingredient,
    pub kind: usize,
    pub amount: i32,
}

impl Conversion {
    fn parse(host: &dyn Host, j: &Value) -> Option<Self> {
        Some(Conversion {
            input: host.recipe_ingredient(&j["input"]["ingredient"])?,
            kind: infuser::type_index(j["output"]["id"].as_str()?)?,
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
}

impl Recipes {
    /// The recipes of a processing machine's type.
    pub fn processing(&mut self, host: &dyn Host, kind: &'static str) -> &[Recipe] {
        self.processing.entry(kind).or_default().refresh(host, kind, Recipe::parse)
    }

    /// The chemical conversions into infuse types.
    pub fn conversion(&mut self, host: &dyn Host) -> &[Conversion] {
        self.conversion.refresh(host, infuser::CONVERSION, Conversion::parse)
    }
}
