use serde::{Deserialize, Serialize};

use crate::models::WikiRouteResolution;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct WikiRouteResolutionResourceData {
    pub item: WikiRouteResolution,
}
