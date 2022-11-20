use std::str::FromStr;

use common::{
    packet::{Amount, UserId},
    socket::SocketError,
};

/// Orden de cafe
#[derive(Clone)]
pub struct Coffee {
    pub name: String,
    pub user_id: UserId,
    pub cost: Amount,
}

/// Tipo de orden.
pub enum Order {
    Sale(Coffee),
    Recharge(Amount, UserId),
}

const ERR_UNKNOWN_TYPE: &str = "Unknown order type";
const ERR_MISSING_FIELDS: &str = "Invalid amount of fields for this order type";

impl FromStr for Order {
    type Err = SocketError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splitted: Vec<&str> = s.split(',').map(str::trim).collect();

        match splitted[0] {
            "sale" => {
                if splitted.len() != 4 {
                    return Err(SocketError::new(ERR_MISSING_FIELDS));
                }
                let coffee = Coffee {
                    name: splitted[1].to_string(),
                    user_id: splitted[2].parse()?,
                    cost: Amount::from_str(splitted[3])?,
                };
                Ok(Order::Sale(coffee))
            }
            "recharge" => {
                if splitted.len() != 3 {
                    return Err(SocketError::new(ERR_MISSING_FIELDS));
                }
                let amount = Amount::from_str(splitted[1])?;
                let user_id = splitted[2].parse()?;
                Ok(Order::Recharge(amount, user_id))
            }
            _ => Err(SocketError::new(ERR_UNKNOWN_TYPE)),
        }
    }
}
