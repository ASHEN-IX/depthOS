use super::*;
use crate::domain::{World, WorldBranch, WorldId};
#[test]
fn hierarchy_overflow_is_explicitly_unavailable_instead_of_truncated() {
    let tree = |count| Universe {
        worlds: (0..count)
            .map(|_| WorldBranch {
                world: World {
                    id: WorldId(Uuid::new_v4()),
                    name: "World".into(),
                    description: None,
                    sort_order: 0,
                    visual_identity: None,
                    artwork_identifier: None,
                    is_archived: false,
                    archived_at_ms: None,
                    version: 1,
                    created_at_ms: 0,
                    updated_at_ms: 0,
                },
                workspaces: vec![],
            })
            .collect(),
    };
    assert_eq!(hierarchy(tree(MAX_ROWS)).unwrap().len(), MAX_ROWS);
    assert!(hierarchy(tree(MAX_ROWS + 1)).is_none());
}
