use serde::{Deserialize, Serialize};

use crate::models::{DriveEventReceipt};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DriveEventReceiptResourceData {
    pub item: DriveEventReceipt,
}
