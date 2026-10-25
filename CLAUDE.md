## Spec Kit + Specwright
- Start a task with `pack_context` (with the feature id), or `get_artifact` for one section. Do not read whole spec files.
- Before saying a task is done, call `check_against_spec` with your diff and report any unmet requirement or out-of-scope change.
- After verifying a task, call `set_task_status`. Record non-obvious choices with `log_decision`.
- Save conventions and pitfalls with `save_memory`; check `search_memory` before changing an approach.