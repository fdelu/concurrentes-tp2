use std::str::FromStr;

use common::{packet::Amount, socket::SocketError};

#[derive(Clone)]
pub struct Coffee {
    pub name: String,
    pub cost: Amount,
}

pub enum Order {
    Sale(Coffee),
    Recharge(Amount),
}

const ERR_UNKNOWN_TYPE: &str = "Unknown order type";
const ERR_MISSING_FIELDS: &str = "Invalid amount of fields for this order type";

impl FromStr for Order {
    type Err = SocketError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splitted: Vec<&str> = s.split(',').collect();

        match splitted[0] {
            "sale" => {
                if splitted.len() != 3 {
                    return Err(SocketError::new(ERR_MISSING_FIELDS));
                }
                let coffee = Coffee {
                    name: splitted[1].to_string(),
                    cost: Amount::from_str(splitted[2])?,
                };
                Ok(Order::Sale(coffee))
            }
            "recharge" => {
                if splitted.len() != 2 {
                    return Err(SocketError::new(ERR_MISSING_FIELDS));
                }
                let amount = Amount::from_str(splitted[1])?;
                Ok(Order::Recharge(amount))
            }
            _ => Err(SocketError::new(ERR_UNKNOWN_TYPE)),
        }
    }
}
