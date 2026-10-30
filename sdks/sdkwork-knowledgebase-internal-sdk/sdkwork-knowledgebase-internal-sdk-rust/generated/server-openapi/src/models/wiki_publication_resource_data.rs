use serde::{Deserialize, Serialize};

use crate::models::WikiPublication;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct WikiPublicationResourceData {
    pub item: WikiPublication,
}
