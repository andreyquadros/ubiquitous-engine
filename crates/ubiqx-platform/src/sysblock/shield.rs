//! The unprivileged side of the system layer: [`NativeSystemShield`] looks at the machine
//! itself and has every change made by the helper, which it starts through the operating
//! system's own prompt.

use std::path::PathBuf;
use std::sync::Arc;

use ubiqx_core::nofap::{
    reason, ApplyOutcome, ObserveOpts, PlannedChange, RemoveScope, SystemCapabilities,
    SystemObservation, SystemPlan, SystemShield, Tier, TierCapability,
};
use ubiqx_core::{CoreError, CoreResult, ProductId};
use ubiqx_nofap::gate::{self, Gate};
use ubiqx_nofap::hosts::CANARY_NAME;
use ubiqx_nofap::matcher;
use ubiqx_nofap::plan::{self, EXIT_APPLIED, EXIT_GATED, EXIT_INVALID_PLAN, EXIT_PARTIAL};
use ubiqx_nofap::resolvers::PROBE_NAME;

use super::elevate::RunnerElevator;
use super::{helper, native_backend, observe, state, sys};
use super::{ElevatedExit, Elevator, Fs, NameLookup, OsBackend, Runner};

/// Set by the AppImage runtime; root cannot read the user's mount, so the helper cannot
/// be started with administrator rights from there.
pub const APPIMAGE_ENV: &str = "APPIMAGE";

/// The system layer of this machine: implements [`SystemShield`] over an [`OsBackend`] and
/// the four ports. `apply` and `remove` re-run `exe` elevated; everything else is
/// unprivileged.
pub struct NativeSystemShield {
    os: Box<dyn OsBackend>,
    runner: Arc<dyn Runner>,
    fs: Arc<dyn Fs>,
    elevator: Arc<dyn Elevator>,
    names: Arc<dyn NameLookup>,
    exe: PathBuf,
    owner: ProductId,
    gate: Gate,
    appimage: bool,
}

impl NativeSystemShield {
    /// Builds the shield from explicit parts (tests use fakes for every port). `exe` is the
    /// executable that is re-run elevated; `owner` is the product whose wants `apply`
    /// records; `gate` is the per-OS gate with the tester's override.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        os: Box<dyn OsBackend>,
        runner: Arc<dyn Runner>,
        fs: Arc<dyn Fs>,
        elevator: Arc<dyn Elevator>,
        names: Arc<dyn NameLookup>,
        exe: PathBuf,
        owner: ProductId,
        gate: Gate,
    ) -> Self {
        Self {
            os,
            runner,
            fs,
            elevator,
            names,
            exe,
            owner,
            gate,
            appimage: false,
        }
    }

    /// Marks the app as running from an AppImage: every tier is then reported
    /// `Unsupported { appimage }`, with the manual steps.
    pub fn running_from_appimage(mut self, appimage: bool) -> Self {
        self.appimage = appimage;
        self
    }

    /// The shield of the operating system this binary runs on, with the real ports, the
    /// current executable and the gate read from the environment. `None` on a system
    /// without a back-end.
    pub fn native(owner: ProductId) -> Option<Self> {
        let backend = native_backend()?;
        let os = backend.os();
        let exe = std::env::current_exe().ok()?;
        let unverified = std::env::var(gate::OVERRIDE_ENV).ok();
        let gate = Gate::new(os, gate::parse_override(unverified.as_deref()));
        let appimage = std::env::var_os(APPIMAGE_ENV).is_some_and(|path| !path.is_empty());
        let runner: Arc<dyn Runner> = Arc::new(sys::SystemRunner);
        let elevator = RunnerElevator::new(
            os,
            backend.elevation_tool(),
            backend.child_env(),
            runner.clone(),
            appimage,
        );
        let shield = Self::new(
            backend,
            runner,
            Arc::new(sys::SystemFs),
            Arc::new(elevator),
            Arc::new(sys::SystemLookup),
            exe,
            owner,
            gate,
        );
        Some(shield.running_from_appimage(appimage))
    }

    /// The product whose wants this shield applies.
    pub fn owner(&self) -> ProductId {
        self.owner
    }

    /// Starts the helper elevated and reads what its exit means.
    fn elevated(&self, args: &[String]) -> CoreResult<ApplyOutcome> {
        let unavailable = |code: &str| Ok(ApplyOutcome::Unavailable(code.to_owned()));
        match self.elevator.run_elevated(&self.exe, args) {
            ElevatedExit::Declined => Ok(ApplyOutcome::Declined),
            ElevatedExit::Unavailable(code) => Ok(ApplyOutcome::Unavailable(code)),
            ElevatedExit::Exited(EXIT_APPLIED) => Ok(ApplyOutcome::Applied),
            ElevatedExit::Exited(EXIT_PARTIAL) => {
                // The per-tier report is in the state file.
                let (state, _) = state::read(&*self.os, &*self.fs);
                let failed = state
                    .map(|state| state.report)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|report| !report.ok)
                    .collect();
                Ok(ApplyOutcome::Partial(failed))
            }
            ElevatedExit::Exited(EXIT_GATED) => unavailable(reason::GATED),
            ElevatedExit::Exited(EXIT_INVALID_PLAN) => Err(CoreError::Invalid(
                "the system helper refused the request".into(),
            )),
            ElevatedExit::Exited(_) => unavailable(reason::FAILED),
        }
    }
}

impl SystemShield for NativeSystemShield {
    fn capabilities(&self) -> SystemCapabilities {
        let of = |tier: Tier| {
            if !self.gate.open(tier) {
                TierCapability::Gated
            } else if self.appimage {
                TierCapability::Unsupported {
                    reason: reason::APPIMAGE.to_owned(),
                }
            } else {
                TierCapability::Available
            }
        };
        SystemCapabilities {
            os: Some(self.os.os()),
            hosts: of(Tier::Hosts),
            resolver: of(Tier::Resolver),
            policies: of(Tier::Policies),
            unverified: self.gate.unverified.clone(),
        }
    }

    fn observe(&self, opts: ObserveOpts) -> CoreResult<SystemObservation> {
        let mut observed = observe::observe(&*self.os, &*self.runner, &*self.fs, opts.full);
        if self.appimage && observed.hosts.error.is_none() {
            observed.hosts.error = Some(reason::APPIMAGE.to_owned());
        }
        if opts.canary {
            observed.canary = Some(self.names.lookup_blocked(CANARY_NAME));
        }
        if opts.probe {
            observed.probe_ok = Some(!self.names.lookup(PROBE_NAME).is_empty());
        }
        Ok(observed)
    }

    fn preview(&self, plan: &SystemPlan) -> CoreResult<Vec<PlannedChange>> {
        let observed = self.observe(ObserveOpts::FULL)?;
        let sent = plan::without_gated(plan, &self.gate);
        Ok(plan::preview(&sent, &observed, self.os.os()))
    }

    fn apply(&self, plan: &SystemPlan) -> CoreResult<ApplyOutcome> {
        // Checked here first, so nobody is asked for a password for a plan the helper
        // would refuse. The helper checks again: it trusts nothing it is given.
        match plan::validate(plan, matcher::never_block(), &self.gate) {
            Ok(()) => {}
            Err(plan::PlanError::Gated { .. }) => {
                return Ok(ApplyOutcome::Unavailable(reason::GATED.to_owned()))
            }
            Err(refused) => return Err(CoreError::Invalid(refused.to_string())),
        }
        if !plan.hosts && plan.resolver.is_none() && !plan.policies {
            // Nothing to apply, so nobody is asked for a password: either the gate left
            // nothing of the plan, or the caller means `remove`.
            return if Tier::ALL.iter().all(|tier| self.gate.open(*tier)) {
                Err(CoreError::Invalid("the plan asks for no tier".into()))
            } else {
                Ok(ApplyOutcome::Unavailable(reason::GATED.to_owned()))
            };
        }
        let encoded = plan::encode_plan(plan);
        if encoded.len() > plan::arg_budget(self.os.os()) {
            return Err(CoreError::Invalid(
                "the plan is longer than this system's argument budget".into(),
            ));
        }
        let unverified = self.gate.unverified_arg();
        self.elevated(&helper::apply_args(&encoded, unverified.as_deref()))
    }

    fn remove(&self, owner: ProductId, scope: RemoveScope) -> CoreResult<ApplyOutcome> {
        self.elevated(&helper::remove_args(owner, scope))
    }
}
