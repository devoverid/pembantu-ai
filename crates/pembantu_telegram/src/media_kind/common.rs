use teloxide::types::{MediaKind, MessageCommon};

pub fn process_common(value: &MessageCommon) {
    match &value.media_kind {
        MediaKind::Text(_text) => {
            
        },
        _ => ()
    }
}