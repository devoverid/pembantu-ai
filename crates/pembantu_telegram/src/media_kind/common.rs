use teloxide::types::{MediaKind, MessageCommon};

pub fn process_common(value: &MessageCommon) {
    if let MediaKind::Text(_text) = &value.media_kind {
        
    }
}