 Message {
    id: MessageId(
        7,
    ),
    thread_id: None,
    date: 2024-01-24T14:10:22Z,
    chat: Chat {
        id: ChatId(
            623454872,
        ),
        kind: Private(
            ChatPrivate {
                username: Some(
                    "asawgi",
                ),
                first_name: Some(
                    "Ogi",
                ),
                last_name: None,
                emoji_status_custom_emoji_id: None,
                bio: None,
                has_private_forwards: None,
                has_restricted_voice_and_video_messages: None,
            },
        ),
        photo: None,
        pinned_message: None,
        message_auto_delete_time: None,
        has_hidden_members: false,
        has_aggressive_anti_spam_enabled: false,
    },
    via_bot: None,
    kind: Common(
        MessageCommon {
            from: Some(
                User {
                    id: UserId(
                        623454872,
                    ),
                    is_bot: false,
                    first_name: "Ogi",
                    last_name: None,
                    username: Some(
                        "asawgi",
                    ),
                    language_code: Some(
                        "en",
                    ),
                    is_premium: false,
                    added_to_attachment_menu: false,
                },
            ),
            sender_chat: None,
            author_signature: None,
            forward: None,
            reply_to_message: None,
            edit_date: None,
            media_kind: Text(
                MediaText {
                    text: "test",
                    entities: [],
                },
            ),
            reply_markup: None,
            is_topic_message: false,
            is_automatic_forward: false,
            has_protected_content: false,
        },
    ),
}


Message {
    id: MessageId(
        9,
    ),
    thread_id: None,
    date: 2024-01-24T14:12:45Z,
    chat: Chat {
        id: ChatId(
            -4189889662,
        ),
        kind: Public(
            ChatPublic {
                title: Some(
                    "ogi testing",
                ),
                kind: Group(
                    PublicChatGroup {
                        permissions: None,
                    },
                ),
                description: None,
                invite_link: None,
                has_protected_content: None,
            },
        ),
        photo: None,
        pinned_message: None,
        message_auto_delete_time: None,
        has_hidden_members: false,
        has_aggressive_anti_spam_enabled: false,
    },
    via_bot: None,
    kind: NewChatMembers(
        MessageNewChatMembers {
            new_chat_members: [
                User {
                    id: UserId(
                        6772037775,
                    ),
                    is_bot: true,
                    first_name: "pembantu",
                    last_name: None,
                    username: Some(
                        "PembantuAI_Bot",
                    ),
                    language_code: None,
                    is_premium: false,
                    added_to_attachment_menu: false,
                },
            ],
        },
    ),
}