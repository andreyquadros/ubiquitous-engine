//! Scripted halves of the `nofap` module's ports, for the engine tests and for hosts
//! without a system layer: a system shield that keeps a pretend machine in memory, and a
//! frame source that serves queued frames. Neither touches the real system.

use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};

use image::RgbaImage;
use parking_lot::Mutex;
use ubiqx_core::nofap::{
    ApplyOutcome, BrowserFamily, Frame, FrameOpts, FrameSource, HostsObservation, InterfaceDns,
    ObserveOpts, Os, OwnerWants, PlannedChange, PolicyCreated, PolicyObservation, RemoveScope,
    ResolverObservation, SystemCapabilities, SystemObservation, SystemPlan, SystemShield,
    SystemState, TierCapability, CONFIG_VERSION,
};
use ubiqx_core::{CoreError, CoreResult, ProductId};
use ubiqx_nofap::{hosts, list, matcher, plan, policies, resolvers};

use crate::image_util::{dhash, downscale_and_encode};

/// One call made to the [`MockSystemShield`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemCall {
    Observe(ObserveOpts),
    Preview(SystemPlan),
    Apply(SystemPlan),
    Remove(ProductId, RemoveScope),
}

/// A [`SystemShield`] over a pretend machine: `apply` and `remove` change an observation
/// kept in memory the way the helper changes a real system (state file, hosts block,
/// resolver, policy values), so `ubiqx_nofap::plan::assess` reads `Applied` after an apply
/// and a test can then damage the observation to play drift. Every call is logged.
pub struct MockSystemShield {
    os: Os,
    calls: Mutex<Vec<SystemCall>>,
    capabilities: Mutex<SystemCapabilities>,
    observation: Mutex<SystemObservation>,
    /// What the next applies and removes answer instead of doing the work.
    apply_outcome: Mutex<Option<ApplyOutcome>>,
    remove_outcome: Mutex<Option<ApplyOutcome>>,
    /// What the canary resolves to; `None` follows the machine (the null address while the
    /// block is in the hosts text).
    canary: Mutex<Option<Vec<String>>>,
    probe_ok: Mutex<bool>,
    /// While set, `apply` waits for a message (or for the sender to go away) before it
    /// does anything: the stand-in for the operating system's password prompt.
    hold: Mutex<Option<Receiver<()>>>,
}

impl MockSystemShield {
    /// A clean machine of `os` with every tier available: a hosts file without a block,
    /// automatic DNS on one active connection (systemd-resolved in use on Linux), Chrome
    /// and Firefox installed without policies, no state file.
    pub fn new(os: Os) -> Self {
        let browser = |browser| PolicyObservation {
            browser,
            present: true,
            values: Vec::new(),
            foreign: Vec::new(),
            error: None,
        };
        let linux = os == Os::Linux;
        let observation = SystemObservation {
            hosts: HostsObservation {
                text: Some("127.0.0.1 localhost\n".into()),
                error: None,
            },
            resolver: ResolverObservation {
                interfaces: if linux {
                    Vec::new()
                } else {
                    vec![InterfaceDns {
                        id: if os == Os::Windows { "12" } else { "Wi-Fi" }.into(),
                        name: "Wi-Fi".into(),
                        active: true,
                        servers: Vec::new(),
                    }]
                },
                dropin: None,
                resolved_active: linux.then_some(true),
                resolv_conf_uses_resolved: linux.then_some(true),
                full: true,
                error: None,
            },
            policies: vec![
                browser(BrowserFamily::Chrome),
                browser(BrowserFamily::Firefox),
            ],
            ..SystemObservation::default()
        };
        Self {
            os,
            calls: Mutex::new(Vec::new()),
            capabilities: Mutex::new(SystemCapabilities {
                os: Some(os),
                hosts: TierCapability::Available,
                resolver: TierCapability::Available,
                policies: TierCapability::Available,
                unverified: Vec::new(),
            }),
            observation: Mutex::new(observation),
            apply_outcome: Mutex::new(None),
            remove_outcome: Mutex::new(None),
            canary: Mutex::new(None),
            probe_ok: Mutex::new(true),
            hold: Mutex::new(None),
        }
    }

    pub fn calls(&self) -> Vec<SystemCall> {
        self.calls.lock().clone()
    }

    pub fn clear_calls(&self) {
        self.calls.lock().clear();
    }

    pub fn set_capabilities(&self, capabilities: SystemCapabilities) {
        *self.capabilities.lock() = capabilities;
    }

    /// The pretend machine as it is now.
    pub fn observation(&self) -> SystemObservation {
        self.observation.lock().clone()
    }

    pub fn set_observation(&self, observation: SystemObservation) {
        *self.observation.lock() = observation;
    }

    /// Changes the pretend machine behind the app's back (drift, a new connection, a
    /// foreign policy).
    pub fn edit_observation(&self, edit: impl FnOnce(&mut SystemObservation)) {
        edit(&mut self.observation.lock());
    }

    /// Makes every following `apply` answer `outcome` and change nothing (a declined
    /// prompt, a missing polkit agent); `None` goes back to doing the work.
    pub fn set_apply_outcome(&self, outcome: Option<ApplyOutcome>) {
        *self.apply_outcome.lock() = outcome;
    }

    /// The same for `remove`.
    pub fn set_remove_outcome(&self, outcome: Option<ApplyOutcome>) {
        *self.remove_outcome.lock() = outcome;
    }

    /// What the canary resolves to from now on; `None` follows the machine.
    pub fn set_canary(&self, answers: Option<Vec<String>>) {
        *self.canary.lock() = answers;
    }

    pub fn set_probe_ok(&self, ok: bool) {
        *self.probe_ok.lock() = ok;
    }

    /// Makes the next `apply` wait, like a password prompt nobody has answered yet, until
    /// something is sent on the returned sender or the sender is dropped.
    pub fn hold_apply(&self) -> Sender<()> {
        let (release, wait) = channel();
        *self.hold.lock() = Some(wait);
        release
    }

    /// Brings the pretend machine to what `owners` want together.
    fn converge(&self, owners: BTreeMap<ProductId, OwnerWants>) {
        let wanted = plan::union(&owners);
        let mut machine = self.observation.lock();
        let previous = machine.state.clone().unwrap_or_default();

        let names = plan::hosts_names(&owners, list::hosts_tier(), matcher::never_block(), self.os);
        let block = wanted.hosts.then(|| hosts::render_block(&names));
        let text = machine.hosts.text.clone().unwrap_or_default();
        let spliced = hosts::splice(text.as_bytes(), block.as_deref());
        machine.hosts.text = Some(String::from_utf8_lossy(&spliced).into_owned());

        let ours = |servers: &[String]| {
            resolvers::RESOLVERS
                .iter()
                .any(|row| resolvers::is_exactly(row.id, servers))
        };
        match wanted.resolver {
            Some(id) if self.os == Os::Linux => {
                machine.resolver.dropin = Some(resolvers::resolved_dropin(id));
            }
            Some(id) => {
                let servers: Vec<String> = resolvers::lookup(id)
                    .addresses()
                    .iter()
                    .map(|address| (*address).to_owned())
                    .collect();
                for interface in machine.resolver.interfaces.iter_mut() {
                    if interface.active {
                        interface.servers.clone_from(&servers);
                    }
                }
            }
            None => {
                machine.resolver.dropin = None;
                for interface in machine.resolver.interfaces.iter_mut() {
                    if ours(&interface.servers) {
                        interface.servers.clear();
                    }
                }
            }
        }

        let mut created = previous.policies_created.clone();
        let conflicts = policies::conflicts(&machine.policies, &created);
        for browser in machine.policies.iter_mut() {
            let family = browser.browser;
            let writable = browser.present
                && browser.error.is_none()
                && policies::applies(family, self.os)
                && !conflicts.contains(&family);
            if wanted.policies && writable {
                for want in policies::desired(family) {
                    if !browser.values.iter().any(|found| found.key == want.key) {
                        created.push(PolicyCreated {
                            browser: family,
                            key: want.key.clone(),
                        });
                        browser.values.push(want);
                    }
                }
            } else if !wanted.policies {
                browser.values.retain(|found| {
                    !created
                        .iter()
                        .any(|made| made.browser == family && made.key == found.key)
                });
            }
        }
        if !wanted.policies {
            created.clear();
        }

        machine.state = (!owners.is_empty()).then(|| SystemState {
            v: CONFIG_VERSION,
            applied_at: previous.applied_at + 1,
            owners,
            hosts: wanted.hosts.then(|| hosts::stamp_of(&names)),
            policies_created: created,
            report: Vec::new(),
        });
        machine.state_error = None;
    }
}

impl SystemShield for MockSystemShield {
    fn capabilities(&self) -> SystemCapabilities {
        self.capabilities.lock().clone()
    }

    fn observe(&self, opts: ObserveOpts) -> CoreResult<SystemObservation> {
        self.calls.lock().push(SystemCall::Observe(opts));
        let mut observation = self.observation.lock().clone();
        if self.os == Os::Windows {
            observation.resolver.full = opts.full;
        }
        if opts.canary {
            let blocked = observation
                .hosts
                .text
                .as_deref()
                .is_some_and(|text| text.contains(hosts::CANARY_NAME));
            let follows = if blocked {
                vec![hosts::NULL_V4.to_owned()]
            } else {
                Vec::new()
            };
            observation.canary = Some(self.canary.lock().clone().unwrap_or(follows));
        }
        if opts.probe {
            observation.probe_ok = Some(*self.probe_ok.lock());
        }
        Ok(observation)
    }

    fn preview(&self, plan: &SystemPlan) -> CoreResult<Vec<PlannedChange>> {
        self.calls.lock().push(SystemCall::Preview(plan.clone()));
        let observation = self.observation.lock().clone();
        Ok(plan::preview(plan, &observation, self.os))
    }

    fn apply(&self, plan: &SystemPlan) -> CoreResult<ApplyOutcome> {
        self.calls.lock().push(SystemCall::Apply(plan.clone()));
        // The receiver is taken out first: nothing is locked while the "prompt" is open.
        let hold = self.hold.lock().take();
        if let Some(wait) = hold {
            // An error only means the sender is gone, which releases the call as well.
            let _ = wait.recv();
        }
        if let Some(outcome) = self.apply_outcome.lock().clone() {
            return Ok(outcome);
        }
        let state = self.observation.lock().state.clone();
        self.converge(plan::owners_after(state.as_ref(), plan));
        Ok(ApplyOutcome::Applied)
    }

    fn remove(&self, owner: ProductId, scope: RemoveScope) -> CoreResult<ApplyOutcome> {
        self.calls.lock().push(SystemCall::Remove(owner, scope));
        if let Some(outcome) = self.remove_outcome.lock().clone() {
            return Ok(outcome);
        }
        let mut owners = self
            .observation
            .lock()
            .state
            .clone()
            .map(|state| state.owners)
            .unwrap_or_default();
        match scope {
            RemoveScope::All => {
                owners.remove(&owner);
            }
            // As the helper: DNS goes back to automatic for every owner.
            RemoveScope::Resolver => {
                for wants in owners.values_mut() {
                    wants.resolver = None;
                }
            }
        }
        self.converge(owners);
        Ok(ApplyOutcome::Applied)
    }
}

/// A [`FrameSource`] that serves queued answers, then a fallback, and records what it was
/// asked for. Queued answers come first.
pub struct ScriptedFrameSource {
    queue: Mutex<VecDeque<CoreResult<Option<Frame>>>>,
    fallback: Mutex<Option<Frame>>,
    calls: Mutex<Vec<(u32, FrameOpts)>>,
}

impl Default for ScriptedFrameSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptedFrameSource {
    /// Serves [`sample_frame(1)`](Self::sample_frame) whenever the queue is empty.
    pub fn new() -> Self {
        Self::always(Some(Self::sample_frame(1)))
    }

    /// Serves `frame` whenever the queue is empty (`None`: there is never a usable frame).
    pub fn always(frame: Option<Frame>) -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            fallback: Mutex::new(frame),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Queues one answer.
    pub fn push(&self, answer: CoreResult<Option<Frame>>) {
        self.queue.lock().push_back(answer);
    }

    pub fn set_fallback(&self, frame: Option<Frame>) {
        *self.fallback.lock() = frame;
    }

    /// Every `(window id, options)` asked for, in order.
    pub fn calls(&self) -> Vec<(u32, FrameOpts)> {
        self.calls.lock().clone()
    }

    /// A real, small JPEG frame, 640 pixels wide. Frames of different seeds are different
    /// pictures with different hashes; the same seed always gives the same frame.
    pub fn sample_frame(seed: u32) -> Frame {
        let bands = 2 + seed % 6;
        let picture = RgbaImage::from_fn(640, 400, |x, y| {
            let lit = (x * bands / 640 + y * (1 + seed % 3) / 400 + seed) % 2 == 0;
            let shade = if lit { 215 } else { 30 };
            image::Rgba([shade, shade.wrapping_add((seed % 40) as u8), shade, 255])
        });
        let dhash = dhash(&picture);
        let image = downscale_and_encode(picture, 640).expect("a 640x400 picture always encodes");
        Frame { image, dhash }
    }

    /// An error to queue, for the path where the capture itself fails.
    pub fn denied() -> CoreError {
        CoreError::Permission("screen recording".into())
    }
}

impl FrameSource for ScriptedFrameSource {
    fn frame(&self, window_id: u32, opts: FrameOpts) -> CoreResult<Option<Frame>> {
        self.calls.lock().push((window_id, opts));
        if let Some(answer) = self.queue.lock().pop_front() {
            return answer;
        }
        Ok(self.fallback.lock().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ubiqx_core::nofap::{Issue, NofapConfig, ResolverId, TierReport, TierState};
    use ubiqx_nofap::gate::Gate;

    fn plan_of(owner: ProductId, extra: &[&str]) -> SystemPlan {
        SystemPlan {
            v: CONFIG_VERSION,
            owner,
            hosts: true,
            extra: extra.iter().map(|name| (*name).to_owned()).collect(),
            resolver: Some(ResolverId::CloudflareFamily),
            policies: true,
        }
    }

    fn config_for(plan: &SystemPlan) -> NofapConfig {
        let mut config = NofapConfig::default();
        config.system.hosts = plan.hosts;
        config.system.resolver = plan.resolver;
        config.system.policies = plan.policies;
        config
    }

    fn assess(shield: &MockSystemShield, os: Os, plan: &SystemPlan) -> plan::SystemAssessment {
        let gate = Gate {
            os,
            unverified: ubiqx_core::nofap::Tier::ALL.to_vec(),
        };
        let observation = shield
            .observe(ObserveOpts {
                full: true,
                canary: true,
                probe: true,
            })
            .unwrap();
        plan::assess(
            &plan::AssessInput {
                config: &config_for(plan),
                plan,
                os,
                gate: &gate,
                hosts_tier: list::hosts_tier(),
                probe_fail_streak: 0,
            },
            &observation,
        )
    }

    #[test]
    fn mock_system_apply_then_observe_reads_applied_on_every_os() {
        for os in Os::ALL {
            let shield = MockSystemShield::new(os);
            let plan = plan_of(ProductId::Ubiqx, &["blocked.example"]);
            let before = assess(&shield, os, &plan);
            assert_eq!(before.hosts, TierState::NotApplied, "{os:?}");
            assert_eq!(shield.apply(&plan).unwrap(), ApplyOutcome::Applied);
            let after = assess(&shield, os, &plan);
            assert_eq!(after.hosts, TierState::Applied, "{os:?}");
            assert_eq!(after.resolver, TierState::Applied, "{os:?}");
            assert_eq!(after.policies, TierState::Applied, "{os:?}");
            assert!(!after.issues.contains(&Issue::HostsNotHonoured), "{os:?}");
            let text = shield.observation().hosts.text.unwrap();
            assert!(text.starts_with("127.0.0.1 localhost\n"));
            assert!(text.contains("0.0.0.0 canary.ubiqx-shield.invalid blocked.example"));
        }
    }

    #[test]
    fn mock_system_remove_returns_the_machine_to_its_start() {
        for os in Os::ALL {
            let shield = MockSystemShield::new(os);
            let clean = shield.observation();
            shield.apply(&plan_of(ProductId::Ubiqx, &[])).unwrap();
            shield.apply(&plan_of(ProductId::Shield, &[])).unwrap();
            // The other product still wants everything.
            shield.remove(ProductId::Ubiqx, RemoveScope::All).unwrap();
            let kept = shield.observation();
            assert!(kept.hosts.text.unwrap().contains(hosts::BEGIN_PREFIX));
            assert_eq!(kept.state.unwrap().owners.len(), 1);
            shield.remove(ProductId::Shield, RemoveScope::All).unwrap();
            assert_eq!(shield.observation(), clean, "{os:?}");
        }
    }

    #[test]
    fn mock_system_remove_resolver_leaves_the_hosts_block() {
        let shield = MockSystemShield::new(Os::Macos);
        let plan = plan_of(ProductId::Shield, &[]);
        shield.apply(&plan).unwrap();
        shield
            .remove(ProductId::Shield, RemoveScope::Resolver)
            .unwrap();
        let machine = shield.observation();
        assert!(machine.hosts.text.unwrap().contains(hosts::BEGIN_PREFIX));
        assert!(machine.resolver.interfaces[0].servers.is_empty());
        let wants = &machine.state.unwrap().owners[&ProductId::Shield];
        assert!(wants.hosts && wants.resolver.is_none());
    }

    #[test]
    fn mock_system_remove_resolver_takes_it_off_for_both_owners() {
        let shield = MockSystemShield::new(Os::Windows);
        shield.apply(&plan_of(ProductId::Ubiqx, &[])).unwrap();
        shield.apply(&plan_of(ProductId::Shield, &[])).unwrap();
        shield
            .remove(ProductId::Ubiqx, RemoveScope::Resolver)
            .unwrap();
        let machine = shield.observation();
        assert!(machine.resolver.interfaces[0].servers.is_empty());
        let state = machine.state.unwrap();
        assert_eq!(state.owners.len(), 2);
        assert!(state.owners.values().all(|wants| wants.resolver.is_none()));
    }

    #[test]
    fn mock_system_logs_calls_and_follows_its_script() {
        let shield = MockSystemShield::new(Os::Linux);
        let plan = plan_of(ProductId::Ubiqx, &[]);
        shield.set_apply_outcome(Some(ApplyOutcome::Declined));
        assert_eq!(shield.apply(&plan).unwrap(), ApplyOutcome::Declined);
        assert!(
            shield.observation().state.is_none(),
            "a declined prompt changes nothing"
        );
        let partial = ApplyOutcome::Partial(vec![TierReport {
            tier: ubiqx_core::nofap::Tier::Resolver,
            ok: false,
            reason: Some("unreachable".into()),
        }]);
        shield.set_remove_outcome(Some(partial.clone()));
        assert_eq!(
            shield.remove(ProductId::Ubiqx, RemoveScope::All).unwrap(),
            partial
        );
        assert!(!shield.preview(&plan).unwrap().is_empty());
        shield.observe(ObserveOpts::QUICK).unwrap();
        assert_eq!(
            shield.calls(),
            vec![
                SystemCall::Apply(plan.clone()),
                SystemCall::Remove(ProductId::Ubiqx, RemoveScope::All),
                SystemCall::Preview(plan),
                SystemCall::Observe(ObserveOpts::QUICK),
            ]
        );
        shield.clear_calls();
        assert!(shield.calls().is_empty());
    }

    #[test]
    fn mock_system_looks_names_up_only_when_asked() {
        let shield = MockSystemShield::new(Os::Windows);
        let quick = shield.observe(ObserveOpts::QUICK).unwrap();
        assert_eq!((quick.canary, quick.probe_ok), (None, None));
        assert!(!quick.resolver.full);
        let asked = ObserveOpts {
            full: true,
            canary: true,
            probe: true,
        };
        let full = shield.observe(asked).unwrap();
        assert!(full.resolver.full);
        assert_eq!(full.canary, Some(Vec::new()), "no block, no answer");
        assert_eq!(full.probe_ok, Some(true));
        shield.set_canary(Some(vec!["203.0.113.7".into()]));
        shield.set_probe_ok(false);
        let scripted = shield.observe(asked).unwrap();
        assert_eq!(scripted.canary, Some(vec!["203.0.113.7".to_owned()]));
        assert_eq!(scripted.probe_ok, Some(false));
    }

    #[test]
    fn mock_system_apply_can_be_held_like_a_password_prompt() {
        let shield = std::sync::Arc::new(MockSystemShield::new(Os::Linux));
        let release = shield.hold_apply();
        let worker = {
            let shield = shield.clone();
            std::thread::spawn(move || shield.apply(&plan_of(ProductId::Ubiqx, &[])).unwrap())
        };
        // The call is logged at once and nothing is locked while it waits.
        while shield.calls().is_empty() {
            std::thread::yield_now();
        }
        assert!(shield.observation().state.is_none());
        assert!(shield.observe(ObserveOpts::QUICK).is_ok());
        release.send(()).unwrap();
        assert_eq!(worker.join().unwrap(), ApplyOutcome::Applied);
        assert!(shield.observation().state.is_some());
    }

    #[test]
    fn scripted_frames_come_from_the_queue_then_the_fallback() {
        let source = ScriptedFrameSource::new();
        let opts = FrameOpts {
            max_edge: 640,
            crop_top_frac: 0.12,
        };
        source.push(Ok(None));
        source.push(Err(ScriptedFrameSource::denied()));
        source.push(Ok(Some(ScriptedFrameSource::sample_frame(7))));
        assert_eq!(source.frame(5, opts).unwrap(), None);
        assert!(matches!(
            source.frame(5, opts),
            Err(CoreError::Permission(_))
        ));
        assert_eq!(
            source.frame(6, opts).unwrap(),
            Some(ScriptedFrameSource::sample_frame(7))
        );
        assert_eq!(
            source.frame(6, opts).unwrap(),
            Some(ScriptedFrameSource::sample_frame(1))
        );
        source.set_fallback(None);
        assert_eq!(source.frame(6, opts).unwrap(), None);
        assert_eq!(source.calls().len(), 5);
        assert_eq!(source.calls()[0], (5, opts));
        assert_eq!(
            ScriptedFrameSource::always(None).frame(1, opts).unwrap(),
            None
        );
    }

    #[test]
    fn sample_frames_are_real_pictures_with_distinct_hashes() {
        let one = ScriptedFrameSource::sample_frame(1);
        let two = ScriptedFrameSource::sample_frame(2);
        assert_eq!(one, ScriptedFrameSource::sample_frame(1));
        assert_eq!(one.image.width, 640);
        assert!(one.image.bytes.starts_with(&[0xFF, 0xD8]));
        assert!(image::load_from_memory(&one.image.bytes).is_ok());
        assert_ne!(one.dhash, two.dhash);
    }
}
