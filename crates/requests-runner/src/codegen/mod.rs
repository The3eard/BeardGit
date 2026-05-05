//! "Copy as ..." code generators. Each target produces a single string
//! suitable for clipboard / copy-paste.

pub mod curl;
pub mod fetch;
pub mod httpie;
pub mod wget;

use crate::types::{CodegenTarget, ResolvedRequest};

pub fn copy_as(req: &ResolvedRequest, target: CodegenTarget) -> String {
    match target {
        CodegenTarget::Curl => curl::generate(req),
        CodegenTarget::Fetch => fetch::generate(req),
        CodegenTarget::Httpie => httpie::generate(req),
        CodegenTarget::Wget => wget::generate(req),
    }
}
