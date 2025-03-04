use potion::HtmlError;
use sqlx::{FromRow, Pool, Postgres, QueryBuilder};

use crate::{
    authentication::permissions::ActionType,
    cryptography::generate_access_token,
    error::QueryError,
    schema::{Cabinet, CabinetMember, CabinetMixer, CabinetMixerOwned, CabinetProduct},
};

use crate::jwt::SessionData;

use super::{get_incredient, get_product, get_user_by_id};

