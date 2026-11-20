# Phase 4.4 Management Contract Specification

The `depthos manage exec` command exposes a structured, transactional JSON contract for application hierarchy management.

## Transport & Execution Model

- Invoked via CLI argument `--request '<json>'` or read from `stdin` when `--request -` or omitted.
- Communicates using UTF-8 JSON.
- Every mutating request is processed within a single SQLite transaction.
- When an operation succeeds, changes are committed and an audit entry is recorded in `hierarchy_audit_log`. On any error, changes are rolled back completely.

## Request Envelope

```json
{
  "operationId": "optional-client-uuid",
  "action": "create | update | reorder | archive | restore | getArchived | getAssociationImpact | inspectProject | getPreference | setPreference | saveUserPreferences",
  "entity": "world | workspace | desktop",
  "id": "target-uuid",
  "parentId": "parent-uuid",
  "expectedVersion": 1,
  "payload": {}
}
```

## Response Envelope

```json
{
  "success": true,
  "operationId": "optional-client-uuid",
  "id": "affected-record-uuid",
  "version": 2,
  "data": {},
  "error": null,
  "code": null
}
```

### Error Codes

| Code | HTTP / Meaning | Description |
|---|---|---|
| `invalid_request` | 400 Bad Request | Malformed JSON or unparseable payload |
| `invalid_metadata` | 422 Unprocessable | Empty name, invalid parent, out-of-bounds parameter |
| `not_found` | 404 Not Found | Referenced entity or parent ID does not exist |
| `conflict` | 409 Conflict | Optimistic concurrency mismatch (`expectedVersion` != `actual_version`) |
| `sqlite_error` | 500 Internal | Database constraint violation or disk I/O failure |

## Actions and Payloads

### 1. `create`
- **World**: `entity: "world"`, payload: `{"name": str, "description": opt_str, "visualIdentity": opt_str, "artwork": opt_str}`
- **Workspace**: `entity: "workspace"`, `parentId: world_id`, payload: `{"name": str, "description": opt_str, "rootDirectory": opt_str, "visualIdentity": opt_str}`
- **Desktop**: `entity: "desktop"`, `parentId: workspace_id`, payload: `{"name": str, "role": opt_str, "visualIdentity": opt_str}`

### 2. `update`
Requires `id` and `expectedVersion`.
- **World**: payload includes updated `name`, `description`, `visualIdentity`, or `artwork`.
- **Workspace**: payload includes updated `name`, `description`, `rootDirectory`, or `visualIdentity`.
- **Desktop**: payload includes updated `name`, `role`, or `visualIdentity`.
- Returns incremented `version` on success; returns `code: "conflict"` if the database version disagrees.

### 3. `reorder`
- `entity`: `"world" | "workspace" | "desktop"`
- `parentId`: required for workspace and desktop
- `payload`: `{"ids": ["uuid-1", "uuid-2", ...]}`
- Atomically updates `sort_order` and increments `version` for all specified siblings.

### 4. `archive` and `restore`
- `archive`: Sets `is_archived = 1`. Requires `expectedVersion`. Does not delete child entities or durable association rules.
- `restore`: Sets `is_archived = 0`. Increments `version`.

### 5. `inspectProject`
- `payload`: `{"path": "/absolute/path"}`
- Returns filesystem status (directory check, owner read permissions) and non-interactive Git metadata (`branch`, `commit`, `isRepo`).
