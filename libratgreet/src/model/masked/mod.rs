use zeroize::Zeroize;

#[derive(Default)]
pub struct MaskedString {
    pub value: String,
    pub mask: Option<String>,
}

impl MaskedString {
    pub fn from(value: String, mask: Option<String>) -> MaskedString {
        MaskedString { value, mask }
    }

    pub fn get(&self) -> &str {
        match self.mask {
            Some(ref mask) => mask,
            None => &self.value,
        }
    }

    pub fn zeroize(&mut self) {
        self.value.zeroize();

        if let Some(ref mut mask) = self.mask {
            mask.zeroize();
        }

        self.mask = None;
    }
}

#[cfg(test)]
mod tests;
