Change harness-notify's notification settings when the operator asks in
plain language (e.g. "turn off the sound", "mute subagent notifications",
"set quiet hours from 10pm to 8am", "what are my current settings").

Translate the request into one of these calls, then run it:

- `harness-notify config get <key>` - read one setting.
- `harness-notify config set <key> <value>` - change one setting.
- `harness-notify config list` - show every current setting.

Valid keys and their values:

| Key | Values | Meaning |
|---|---|---|
| `events.done` | `true`/`false` | notify when a task/session finishes |
| `events.attention` | `true`/`false` | notify when input/a decision is needed |
| `events.subagent_done` | `true`/`false` | notify when a subagent finishes |
| `session.include_name` | `true`/`false` | include the harness name in the message |
| `session.format` | `name` or `path` | how the session is labeled, if included |
| `sound.enabled` | `true`/`false` | play a sound with the notification |
| `dnd.enabled` | `true`/`false` | enable quiet hours |
| `dnd.start` | `HH:MM` | quiet hours start (local time) |
| `dnd.end` | `HH:MM` | quiet hours end (local time, may cross midnight) |

Example: "mute subagent notifications" -> `harness-notify config set events.subagent_done false`.
