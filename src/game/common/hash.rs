pub trait Hashable {
    fn hash(&self) -> u32;
}

pub fn hash(value: impl Hashable) -> u32 {
    value.hash()
}

impl Hashable for &[u8] {
    fn hash(&self) -> u32 {
        shadow_company_tools::common::hash(self)
    }
}

impl Hashable for &str {
    fn hash(&self) -> u32 {
        let bytes = self.as_bytes();
        debug_assert!(bytes.is_ascii());
        shadow_company_tools::common::hash(bytes)
    }
}
