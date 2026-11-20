//! The one command both shells expose: `nofap(command: NofapCommand)`. Its body is
//! [`handle`], so the wire contract is tested here, without a shell. The request and
//! reply types are `ubiqx_core::nofap_ipc`; the reply of each command is documented on
//! its variant.
//!
//! [`handle`] blocks (a database, and for three commands the operating system's password
//! prompt): a shell calls it on the blocking pool.

use serde::Serialize;
use serde_json::Value;
use ubiqx_core::nofap_ipc::NofapCommand;
use ubiqx_core::{CoreError, CoreResult};

use super::NofapService;

fn reply<T: Serialize>(value: T) -> CoreResult<Value> {
    serde_json::to_value(value).map_err(|error| CoreError::Other(error.to_string()))
}

/// Answers one request of the page or of the card window.
pub fn handle(service: &NofapService, command: NofapCommand) -> CoreResult<Value> {
    match command {
        NofapCommand::View => reply(service.view()?),
        NofapCommand::Enable(options) => reply(service.enable(options)?),
        NofapCommand::ConsentVision(args) => reply(service.consent_vision(args)?),
        NofapCommand::Request(change) => reply(service.request(change)?),
        NofapCommand::ConfirmPending { id } => reply(service.confirm_pending(&id)?),
        NofapCommand::CancelPending { id } => reply(service.cancel_pending(&id)?),
        NofapCommand::SystemPreview => reply(service.system_preview()?),
        NofapCommand::SystemApply => reply(service.system_apply()?),
        NofapCommand::SystemRemove => reply(service.system_remove()?),
        NofapCommand::RestoreNetwork => reply(service.restore_network()?),
        NofapCommand::Sites => reply(service.sites()),
        NofapCommand::Support => reply(service.support()?),
        NofapCommand::Record(event) => reply(service.record(event)?),
        NofapCommand::Card { id } => reply(service.card(&id)),
        NofapCommand::CardAction { id, action } => reply(service.card_action(&id, action)?),
        NofapCommand::SampleFrame => reply(service.sample_frame()?),
        NofapCommand::EraseHistory => reply(service.erase_history()?),
        NofapCommand::Reset => reply(service.reset()?),
        NofapCommand::Export => reply(service.export()?),
        NofapCommand::TestCard => {
            service.test_card();
            Ok(Value::Null)
        }
    }
}
