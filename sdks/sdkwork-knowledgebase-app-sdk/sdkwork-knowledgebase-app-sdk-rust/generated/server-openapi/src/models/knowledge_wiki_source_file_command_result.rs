use serde::{Deserialize, Serialize};

use crate::models::{KnowledgeWikiPublication, KnowledgeWikiSourceFile};

/// Updated publication and source-file state after a Wiki command.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KnowledgeWikiSourceFileCommandResult {
    pub publication: KnowledgeWikiPublication,

    #[serde(rename = "sourceFile")]
    pub source_file: KnowledgeWikiSourceFile,
}
