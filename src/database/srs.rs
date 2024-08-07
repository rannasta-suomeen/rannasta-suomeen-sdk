use std::collections::VecDeque;

use potion::TypeError;
use serde_json::Value;

use crate::schema::UnitType;

#[derive(Debug, Clone)]
pub struct StandardRecipeSyntax {
    pub name: String,
    pub parts: Vec<StandardRecipePart>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StandardRecipePart {
    pub amount: i32,
    pub unit: UnitType,
    pub incredient_name: String,
}

impl TryFrom<String> for StandardRecipeSyntax {
    type Error = potion::error::TypeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let data: VecDeque<&str> = value.split("#").collect();
        let mut recipe_parts: VecDeque<&str> = data.get(0).unwrap_or(&"").split("|").collect();
        let recipe_tags: VecDeque<&str> = data.get(1).unwrap_or(&"").split("/").collect();

        let mut parts: Vec<StandardRecipePart> = vec![];
        if recipe_parts.len() <= 1 {
            return Err(TypeError::new("Invalid syntax"));
        };

        let name = recipe_parts.pop_front().unwrap();
        let mut i = 1;

        let mut tmp = StandardRecipePart {
            amount: 0,
            unit: UnitType::Cl,
            incredient_name: String::new(),
        };

        for part in recipe_parts.iter() {
            match i {
                1 => {
                    let amount = part.parse::<i32>().ok();
                    if amount.is_none() {
                        return Err(TypeError::new("Invalid syntax; Invalid amount"));
                    }

                    tmp.amount = amount.unwrap();
                }
                2 => {
                    let value = Value::String(String::from(*part));
                    let unit = UnitType::try_from(value).ok();
                    if unit.is_none() {
                        return Err(TypeError::new("Invalid syntax; Invalid unit"));
                    }

                    tmp.unit = unit.unwrap();
                }
                3 => {
                    tmp.incredient_name = String::from(*part);

                    parts.push(tmp);

                    tmp = StandardRecipePart {
                        amount: 0,
                        unit: UnitType::Cl,
                        incredient_name: String::new(),
                    };

                    i = 0;
                }
                _ => unreachable!(),
            }

            i += 1;
        }

        Ok(Self {
            name: name.to_string(),
            parts,
            tags: recipe_tags.iter().map(|tag| tag.to_string()).collect(),
        })
    }
}
