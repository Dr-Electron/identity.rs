use serde::{Deserialize, Serialize};

use identity_core::{common::Timestamp, did::DID};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct DIDProof {
    /// A DID URL referencing a DID document key used for signature creation.
    pub id: DID,
    /// A timestamp of when the DID proof was created.
    pub created: Timestamp,
    /// The signature value generated by the signature algorithm.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub signature: String,
}

impl DIDProof {
    pub fn new(id: DID) -> Self {
        Self {
            id,
            created: Timestamp::now(),
            signature: String::new(),
        }
    }
}
