//! This is a parser crate for the [CookLang](https://github.com/cooklang/spec).  The main feature is parsing a String into a
//! struct that implements serde and can be easily used from there.
//!
//! The implementation is nearly fully complete. Only image tags are missing. They are just ignored by now.
//!

extern crate pest;
#[macro_use]
extern crate pest_derive;
extern crate indexmap;

use indexmap::IndexMap;
use pest::iterators::Pair;
use pest::Parser;
use std::boxed::Box;
use std::collections::HashMap;
use std::ops::Add;
use std::str::FromStr;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Parser)]
#[grammar = "../CookLang.pest"]
struct CookParser;

/// Includes the raw source, metadata and instructions.
#[derive(Debug, Serialize, Deserialize)]
pub struct Recipe {
    /// Raw source code of the recipe that this struct has been generated from.
    pub source: String,
    /// Contains the metadata of the recipe. Provided in the form of [Metadata].
    pub metadata: Metadata,
    /// Contains reduced instructions.
    ///
    /// For every mentioning of a ingredient there is an @ in replacement. The mentioning directly
    /// links to an [IngredientSpecifier].
    ///
    /// For every mentioning of a cookware there is an # in replacement. The mentioning directly
    /// links to a [String] describing the cookware.
    ///
    /// For every mentioning of a timer there is an ~ in replacement. The mentioning directly links
    /// to a [Timer].
    pub instruction: String,
}

/// The metadata from the recipe is described in this metadata struct.
#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    /// Amount of servings. Is optional.
    pub servings: Option<Vec<usize>>,
    /// Other optional metadata contained in a [HashMap].
    pub ominous: HashMap<String, String>,
    /// Exact description of an [Ingredient] indexed by name.
    pub ingredients: IndexMap<String, Ingredient>,
    /// Ingredient Specifier describing the mentioning of a [Ingredient]. The n-th mention of @
    /// in [Recipe::instruction] is the n-th [IngredientSpecifier] in this [Vec].
    pub ingredients_specifiers: Vec<IngredientSpecifier>,
    /// The n-th mention of # in [Recipe::instruction] is the n-th [String] in this [Vec].
    pub cookware: Vec<String>,
    /// The n-th mention of ~ in [Recipe::instruction] is the n-th [Timer] in this [Vec].
    pub timer: Vec<Timer>,
}

impl Metadata {
    fn add_key_value(&mut self, key: String, value: String) {
        self.ominous.insert(key, value);
    }
}
/// A Timer.
///
/// Describing the timer you have to set in this mentioning in the instructions.
#[derive(Debug, Serialize, Deserialize)]
pub struct Timer {
    /// The number of [Timer::unit]s in this Timer mentioning.
    pub amount: f64,
    /// The unit of this Timer contained in a [String].
    pub unit: String,
}

/// IngredientSpecifier
///
/// References to a [Ingredient] in [Metadata::ingredients] by [String].
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IngredientSpecifier {
    /// Name of the ingredient this specifier references to. Have to be extracted from [Metadata::ingredients].
    pub ingredient: String,
    /// [Amount] to be used in this step.
    pub amount_in_step: Amount,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ingredient {
    /// Name of the ingredient.
    pub name: String,
    /// Uuid is currently not used.
    pub id: Uuid,
    /// Optional [Amount] specifier.
    pub amount: Option<Amount>,
    /// Unit this ingredient is measured in.
    pub unit: Option<String>,
}

/// Specifies the amount of a [Ingredient].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Amount {
    /// Scalable amount.
    ///
    /// To get the needed amount in the step or total needed amount [Amount::Multi::0] has to be
    /// multiplied by the servings.
    Multi(f64),
    /// Static Servings amount.
    Servings(Vec<f64>),
    /// Static amount.
    Single(f64),
}

impl Add for Amount {
    type Output = Amount;

    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Amount::Multi(a) => match rhs {
                Amount::Multi(b) => Amount::Multi(a + b),
                _ => {
                    panic!("Unallowed Addition");
                }
            },
            Amount::Servings(a) => match rhs {
                Amount::Servings(b) => {
                    Amount::Servings(a.iter().zip(b.iter()).map(|e| *e.0 + *e.1).collect())
                }
                _ => {
                    panic!("Unallowed Addition");
                }
            },
            Amount::Single(a) => match rhs {
                Amount::Single(b) => Amount::Single(a + b),
                _ => {
                    panic!("Unallowed Addition");
                }
            },
        }
    }
}


/// Parse the input into a [Recipe].
pub fn parse(inp: &str) -> Result<Recipe, Box<dyn std::error::Error>> {
    let successful_parse: Pair<_> = match CookParser::parse(Rule::cook_lang, inp) {
        Ok(d) => d,
        Err(e) => {
            panic!("{:?}", e);
        }
    }
    .next()
    .unwrap();
    let mut metadata = Metadata {
        servings: None,
        ominous: Default::default(),
        ingredients: IndexMap::new(),
        ingredients_specifiers: vec![],
        cookware: vec![],
        timer: vec![],
    };
    let source = successful_parse.as_str().to_string();
    let mut source_edited = source.clone();
    let metadata_line_iterator = successful_parse.clone().into_inner();
    metadata_line_iterator.for_each(|e| {
        if e.as_rule() == Rule::metadata {
            e.into_inner().for_each(|property| {
                let mut key_value_iterator = property.into_inner();
                let name = key_value_iterator.next().unwrap().as_str();

                if name != "servings" {
                    let value = key_value_iterator.next().unwrap().as_str();
                    metadata.add_key_value(name.to_string(), value.to_string());
                } else {
                    let mut servings = Vec::with_capacity(3);
                    key_value_iterator
                        .next()
                        .unwrap()
                        .into_inner()
                        .for_each(|serving| {
                            // println!("Serving => {:?}", serving);
                            if serving.as_str() != "|" {
                                let serving_number = usize::from_str(serving.as_str())
                                    .expect("Parsing of serving number failed");
                                servings.push(serving_number);
                            }
                        });
                    metadata.servings = Some(servings);
                }
            });
        } else if e.as_rule() == Rule::comment {
            println!("Replacing comment = {}", e.as_str());
            source_edited = source_edited.replace(e.as_str(), "");

        } else {
            // println!("Line => {:?}", e);
            let _line = e.as_str().to_string().clone();
            e.into_inner().for_each(|ingredients_cookware| {
                // println!("Ingredient / Cookware => {:?}", ingredients_cookware);
                if ingredients_cookware.as_rule() == Rule::ingredient {
                    source_edited = source_edited.replace(ingredients_cookware.as_str(), "@");
                    // println!("Ingredient => {:?}", ingredients_cookware);
                    let mut name = String::new();
                    let mut ingredient_amount = None;
                    let mut ingredient_modified = None;
                    let mut ingredient_unit = None;
                    ingredients_cookware
                        .into_inner()
                        .for_each(|ingredient_property| {
                            // println!("Ingredient Property => {:?}", ingredient_property);
                            match ingredient_property.as_rule() {
                                Rule::name => {
                                    name.push_str(ingredient_property.as_str());
                                    name.push(' ');
                                }
                                Rule::text => {
                                    name.push_str(ingredient_property.as_str());
                                    name.push(' ');
                                }
                                Rule::number => {
                                    ingredient_property.into_inner().for_each(
                                        |ingredient_amount_inner| match ingredient_amount.clone() {
                                            None => {
                                                ingredient_amount = Some(Amount::Single(
                                                    usize::from_str(
                                                        ingredient_amount_inner.as_str(),
                                                    )
                                                    .expect("Failed to parse ingredient amount")
                                                        as f64,
                                                ))
                                            }
                                            Some(d) => {
                                                let data_point = usize::from_str(
                                                    ingredient_amount_inner.as_str(),
                                                )
                                                .expect("Failed to parse ingredient amount")
                                                    as f64;
                                                let ingredient_amount_raw = match d {
                                                    Amount::Multi(_) => {
                                                        panic!("This isn't allowed with multiply.")
                                                    }
                                                    Amount::Servings(dd) => {
                                                        let mut res = dd.clone();
                                                        // println!("Res => {:?}", res);
                                                        let last = res.len() - 1;
                                                        if res.get(last).unwrap().clone() == 0.0 {
                                                            let reference =
                                                                res.get_mut(last).unwrap();
                                                            *reference = data_point;
                                                        } else {
                                                            let dat = res.pop().unwrap();
                                                            res.push(dat / data_point);
                                                        }
                                                        // println!("Res => {:?}", res);
                                                        Amount::Servings(res)
                                                    }
                                                    Amount::Single(d) => {
                                                        Amount::Single(d / data_point)
                                                    }
                                                };
                                                ingredient_amount = Some(ingredient_amount_raw);
                                            }
                                        },
                                    );
                                }
                                Rule::ingredient_separator => match ingredient_amount.clone() {
                                    None => {
                                        panic!("This shouldn't have happened.");
                                    }
                                    Some(d) => match d {
                                        Amount::Multi(_) => {
                                            panic!("This shouldn't have happened.")
                                        }
                                        Amount::Servings(dd) => {
                                            let mut res = dd.clone();
                                            res.push(0.0);
                                            ingredient_amount = Some(Amount::Servings(res));
                                        }
                                        Amount::Single(dd) => {
                                            ingredient_amount =
                                                Some(Amount::Servings(vec![dd, 0.0]));
                                        }
                                    },
                                },
                                Rule::modified => {
                                    let modified = ingredient_property
                                        .into_inner()
                                        .next()
                                        .unwrap()
                                        .as_str()
                                        .to_string();
                                    ingredient_modified = Some(modified);
                                }
                                Rule::unit => {
                                    ingredient_unit = Some(ingredient_property.as_str().to_string())
                                }
                                Rule::scaling => {
                                    ingredient_amount = match ingredient_amount.clone() {
                                        Some(d) => match d {
                                            Amount::Single(d) => Some(Amount::Multi(d)),
                                            _ => {
                                                panic!("This shouldn't have happened.")
                                            }
                                        },
                                        None => {
                                            panic!("This shouldn't have happened.")
                                        }
                                    }
                                }
                                _ => {
                                    panic!("That should have happened")
                                }
                            }
                        });
                    if name.len() > 0 {
                        name.pop();
                    }
                    let ingredient_specifier = IngredientSpecifier {
                        ingredient: name.clone(),
                        amount_in_step: match ingredient_amount.clone() {
                            None => Amount::Single(0.0),
                            Some(d) => d,
                        },
                    };
                    metadata
                        .ingredients_specifiers
                        .push(ingredient_specifier.clone());
                    if metadata.ingredients.contains_key(&name) {
                        let mut ingredient = metadata.ingredients.get_mut(&name).unwrap();
                        match ingredient_amount.clone() {
                            None => {}
                            Some(amount) => {
                                ingredient.amount =
                                    Some(ingredient.amount.as_ref().unwrap().clone() + amount);
                            }
                        }
                        if ingredient.unit != ingredient_unit {
                            panic!("Amount of ingredient is inconsistent.")
                        }
                        ingredient.unit = ingredient_unit;
                    } else {
                        let ingredient = Ingredient {
                            name: name.clone(),
                            id: Uuid::new_v4(),
                            amount: ingredient_amount,
                            unit: ingredient_unit,
                        };
                        metadata.ingredients.insert(name.clone(), ingredient);
                    }
                    // println!("Name => {}", name);
                } else if ingredients_cookware.as_rule() == Rule::cookware {
                    source_edited = source_edited.replace(ingredients_cookware.as_str(), "#");
                    // println!("Cookware => {:?}", ingredients_cookware);
                    let mut name = String::new();
                    ingredients_cookware
                        .into_inner()
                        .for_each(|cookware_property| {
                            // println!("Cookware Property => {:?}", cookware_property);
                            name.push_str(cookware_property.as_str());
                            name.push(' ');
                        });
                    name.pop().unwrap();
                    // println!("Name => {}", name);
                    metadata.cookware.push(name);
                } else if ingredients_cookware.as_rule() == Rule::timer {
                    source_edited = source_edited.replace(ingredients_cookware.as_str(), "~");
                    // println!("Timer => {:?}", ingredients_cookware);
                    let mut timer = Timer {
                        amount: 0.0,
                        unit: "".to_string(),
                    };
                    ingredients_cookware
                        .into_inner()
                        .for_each(|timer_property| {
                            // println!("Timer Property => {:?}", timer_property);
                            if timer_property.as_rule() == Rule::number {
                                let amount = usize::from_str(timer_property.as_str())
                                    .expect("Unaple to parse timer duration")
                                    as f64;
                                timer.amount = amount;
                            } else {
                                let unit = timer_property.as_str().to_string();
                                timer.unit = unit;
                            }
                        });
                    metadata.timer.push(timer);
                } else if ingredients_cookware.as_rule() == Rule::comment {
                    println!("Replacing comment {}", ingredients_cookware.as_str());
                    source_edited = source_edited.replace(ingredients_cookware.as_str(), "");
                }
            })
        }
    });
    // println!("{:#?}", successful_parse);
    // println!("Source edited: {}", source_edited);
    // println!("{:#?}", metadata);
    let recipe = Recipe {
        source,
        metadata,
        instruction: source_edited
    };
    Ok(recipe)

}

#[cfg(test)]
mod tests {
    use crate::parse;
    use std::fs::read_to_string;

    #[test]
    fn it_works() {
        let test_rec = String::from(
            "\
>> value: key // This is a comment\n\
// A comment line\n\
>> servings: 1|2|3\n\
Get some @fruit salat ananas{1/2*}(washed) and pull it\n\
Use the #big potato masher{}\n\
Start the timer ~{10%minutes}\n\
",
        );

        let _recipe = parse(&test_rec).unwrap();
    }

    #[test]
    fn coffee_souffle() {
        let test_rec = read_to_string("../spec/examples/Coffee Souffle.cook").unwrap();
        parse(&test_rec).unwrap();
    }

    #[test]
    fn easy_pancakes() {
        let test_rec = read_to_string("../spec/examples/Easy Pancakes.cook").unwrap();
        parse(&test_rec).unwrap();
    }

    #[test]
    fn fried_rice() {
        let test_rec = read_to_string("../spec/examples/Fried Rice.cook").unwrap();
        parse(&test_rec).unwrap();
    }

    #[test]
    fn olivier_salad() {
        let test_rec = read_to_string("../spec/examples/Olivier Salad.cook").unwrap();
        parse(&test_rec).unwrap();
    }
}
