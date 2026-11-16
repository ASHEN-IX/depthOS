//! Private fixture adapter for a display-only KWin preview.
//! The payload cannot construct an ActionRequest, capability, or consent.
use depthos_core::action_security::{Authorizer, Client, Error};
use serde::{Deserialize, Serialize};
use std::{
    error::Error as StdError,
    io::{self, Read},
};
use uuid::Uuid;

const MAX_INPUT: usize = 4096;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlacementPreview {
    source_desktop: String,
    destination_desktop: String,
    activity: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureInput {
    bus_id: Uuid,
    unique_owner: String,
    uid: u32,
    preview: PlacementPreview,
}

#[derive(Serialize)]
struct FixtureOutput {
    authorization: &'static str,
    preview: PlacementPreview,
}

fn deny_and_project(input: FixtureInput) -> Result<FixtureOutput, &'static str> {
    let caller = Client {
        bus_id: input.bus_id,
        unique_name: input.unique_owner,
        uid: input.uid,
    };
    if !matches!(
        Authorizer::new().verify_interaction_claim(&caller, true),
        Err(Error::UnverifiedInteraction)
    ) {
        return Err("production interaction verifier did not refuse");
    }

    let preview = input.preview;
    let labels = [
        &preview.source_desktop,
        &preview.destination_desktop,
        &preview.activity,
    ];
    if labels.iter().any(|label| {
        label.trim().is_empty() || label.len() > 128 || label.chars().any(char::is_control)
    }) || preview.source_desktop == preview.destination_desktop
    {
        return Err("invalid placement preview");
    }
    Ok(FixtureOutput {
        authorization: "UnverifiedInteraction",
        preview,
    })
}

fn main() -> Result<(), Box<dyn StdError>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_INPUT + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_INPUT {
        return Err("preview fixture exceeded input limit".into());
    }
    let result = deny_and_project(serde_json::from_slice(&bytes)?)?;
    serde_json::to_writer(io::stdout().lock(), &result)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> FixtureInput {
        FixtureInput {
            bus_id: Uuid::new_v4(),
            unique_owner: ":1.2".into(),
            uid: 1000,
            preview: PlacementPreview {
                source_desktop: "source".into(),
                destination_desktop: "destination".into(),
                activity: "activity".into(),
            },
        }
    }

    #[test]
    fn plausible_owner_and_placement_produce_a_denial_with_display_data() {
        let result = deny_and_project(input()).unwrap();
        assert_eq!(result.authorization, "UnverifiedInteraction");
        assert_eq!(result.preview.source_desktop, "source");
        assert_eq!(result.preview.destination_desktop, "destination");
        assert_eq!(result.preview.activity, "activity");
    }

    #[test]
    fn rejects_ambiguous_placement_and_extra_fields() {
        let mut input = input();
        input.preview.destination_desktop = input.preview.source_desktop.clone();
        assert!(deny_and_project(input).is_err());
        let extra = r#"{"sourceDesktop":"source","destinationDesktop":"dest","activity":"activity","capability":"claim"}"#;
        assert!(serde_json::from_str::<PlacementPreview>(extra).is_err());
    }
}
