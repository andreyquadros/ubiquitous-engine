# Research: evidence-informed support for the "nofap" module (counselling, features, naming)

Scope: what the mascot UBI should say and do, what the product must not claim, where to send
people who need more than an app, how comparable apps work and fail, whether the name "nofap"
can be used commercially, a message bank, and a first-version feature set.

Date of research: 2026-11-19.

## 0. How to read this report

**Method and its limit.** Everything below comes from web search result summaries. Direct page
fetches were blocked in this session for every host tried (`pmc.ncbi.nlm.nih.gov`, `cvv.org.br`,
`www.slaa.org.br`, `e-psi.cfp.org.br`, `trademarks.justia.com` are outside the session's allowed
network domains), so no primary page, full paper or trademark register was opened. Numbers quoted
from abstracts are marked as such. Anything that will ship in the product (phone numbers, URLs,
trademark status) needs one more check against the primary source by a human.

**Evidence tags**

| Tag | Meaning |
|---|---|
| [V] | Confirmed in this session by search results quoting a primary source (journal abstract, official report, vendor documentation) |
| [S] | Secondary source only (press, aggregator, blog, vendor marketing) |
| [K] | Background knowledge, not re-checked in this session |
| H / M / L | Confidence in the statement: high, medium, low |

**The ten conclusions a designer needs**

1. Do not name anything user-visible "NoFap". It is a registered US trademark of NoFap LLC, the
   owner litigates, and the word carries claims (masturbation abstinence, "reboot") the product
   should not make. See section 5.
2. The user sets the goal (stop or reduce pornography). The product never sets a masturbation or
   sexual-abstinence goal and never diagnoses.
3. The honest pitch is "a barrier plus a calm coach", not a treatment. Evidence for every
   psychological technique in this area is promising but low quality (GRADE low or very low).
4. The best-supported counselling content is ACT/mindfulness-style urge surfing, CBT relapse
   prevention (triggers, if-then plans, stimulus control) and a self-compassionate response to a
   lapse. The message bank in section 7 is built on those three.
5. Never claim: dopamine reset, brain rewiring, a 90-day reboot, cured erectile dysfunction,
   higher testosterone, "superpowers", or any success rate.
6. Streaks: optional, quiet, never a single counter that falls to zero. Show "days that went the
   way you wanted in the last 30" next to it.
7. No accountability partner in version 1. If added later: user-initiated, summary only, never
   screenshots or URLs, never installable to watch someone else.
8. Store nothing intimate on a server. The 2026 Quittr leak (600,000+ users' data) is the
   cautionary case. No screenshots kept, counts only, local only.
9. Disabling the shield should cost time, not be impossible: a user-chosen delay (Plucky model)
   with an urge-surf offer while waiting, and an emergency network-recovery path that never needs
   the delay.
10. Crisis path in Brazil: CVV 188 (free, 24 h) and `cvv.org.br` chat; emergencies 192; ongoing
    care via CAPS (walk-in, SUS) or a CFP-registered psychologist. Outside Brazil:
    `findahelpline.com`.

---

## 1. The clinical picture

### 1.1 ICD-11 6C72, compulsive sexual behaviour disorder (CSBD)

- CSBD is code **6C72** in ICD-11, in the chapter on mental, behavioural or neurodevelopmental
  disorders, grouped with **impulse control disorders**, not with disorders due to addictive
  behaviours. [V, H] The core definition: "a persistent pattern of failure to control intense,
  repetitive sexual impulses or urges resulting in repetitive sexual behaviour". [V, H]
- Features listed in the description: sexual activity becoming a central focus of life to the
  point of neglecting health, personal care or other interests and responsibilities; numerous
  unsuccessful efforts to reduce it; continuing "despite adverse consequences or deriving little
  or no satisfaction from it"; the pattern lasting an extended period (six months or more) and
  causing marked distress or significant impairment. [V for the quoted clause, K for the rest, H]
- The exclusion that matters most for this product: "Distress that is entirely related to moral
  judgments and disapproval about sexual impulses, urges, or behaviours is not sufficient to meet
  this requirement." [V via a 2025 Frontiers in Psychiatry review that reproduces it, H] A 2024
  sexual-medicine review describes this as a deliberate guard against over-pathologising. [V, M]
- Paraphilic disorders are excluded from 6C72. [V, H]
- DSM-5 has no equivalent diagnosis; the proposed "hypersexual disorder" was not adopted. [K, H]
- Pornography use is one behaviour through which CSBD can show; ICD-11 does not define a separate
  "pornography addiction". [K, H]
- Scale: a 42-country survey estimated that close to 5% of respondents were at high risk of CSBD,
  and only 14% of them had ever sought treatment; another 33% wanted to but did not. [S (press
  report of Bőthe et al. 2023, Journal of Behavioral Addictions), M]

**Design consequence.** The app cannot and should not diagnose. It can describe, in plain words,
the pattern that deserves a professional conversation: it has lasted months, you have tried to
cut down several times and could not, and it is costing you sleep, work, study, money or
relationships. Feeling guilty because of your values, with no loss of control, is a different
situation and is not a disorder.

### 1.2 "Problematic pornography use" (PPU)

- PPU is a research term, not a diagnosis. The largest study is Bőthe et al., *Addiction* 2024,
  119(5):928-950: International Sex Survey, **n = 82,243**, **42 countries**, mean age 32.4.
  **3.2%** were at risk on the Problematic Pornography Consumption Scale; the estimate ranges
  from 3.2% to 16.6% depending on the instrument. Men scored highest; no differences by sexual
  orientation. [V (abstract), H]
- Treatment gap in the same study: only **4-10%** of the at-risk group had ever sought treatment;
  a further **21-37%** wanted to and did not, for reasons such as cost. [V (abstract), H] This is
  the audience for a low-barrier self-help tool, and the reason a referral path matters.
- Frequency of use alone does not define a problem. Loss of control and consequences do. [K, H]

### 1.3 Moral incongruence

- Grubbs, Perry, Wilt and Reid, "Pornography Problems Due to Moral Incongruence", *Archives of
  Sexual Behavior* 2019, 48(2):397-415, propose two routes to feeling "addicted": genuine
  dysregulation, and **moral incongruence** (using pornography while morally disapproving of it).
  Their systematic review and a later review of 51 studies (2016-2021) find religiousness, moral
  disapproval and moral incongruence to be strong, robust predictors of self-perceived addiction
  and distress. [V (abstracts), H]
- Grubbs has said the research does not mean pornography-related problems "are not real or are
  only a function of morality". [S, M] The two routes coexist.

**Design consequence.** The mascot must work for both groups without endorsing either story. It
does not tell a religious user their distress is "only guilt", and it does not tell anyone they
are an addict. It asks what the person wants and helps with that. It never amplifies shame,
because shame is the common factor that makes both routes worse.

### 1.4 Contested claims of the NoFap / "reboot" community

| Claim | What the evidence says | Tag |
|---|---|---|
| "90 days of abstinence resets dopamine / reboots the brain" | No clinical trial tests a "reboot". Fernandez, Kuss and Griffiths (*Clinical Psychology Review* 2020, 76:101828) reviewed 47 prospective abstinence studies across six behaviours; only 3 were about pornography, with "some benefits" of short-term abstinence but thin evidence. A 2023 randomised study from the same group found regular users showed no withdrawal symptoms over 7 days. A 2014 review by Ley, Prause and Finn criticised the addiction model as poorly specified (authors have a stake in the debate). | [V abstracts, S for the 2023 and 2014 items], M |
| "Pornography causes erectile dysfunction (PIED), abstinence cures it" | Grubbs and Gola (*Journal of Sexual Medicine* 2019): little or no evidence that pornography use itself relates to erectile function; self-reported *problematic* use correlates with erectile difficulty at the same time point, with no evidence of a causal link over time. Dwulit and Rzymski (*Journal of Clinical Medicine* 2019) reach a similar conclusion from observational studies. The opposing case rests on a 2016 narrative review with clinical reports (Park et al., *Behavioral Sciences*). Prause and Binnie (2022) suggest anxiety about erections explains much of the reported problem. | [V abstracts], M-H |
| "Abstinence raises testosterone" / "superpowers" | The source is Jiang et al. 2003 (*Journal of Zhejiang University Science*), 28 volunteers, a one-day spike to about 145.7% of baseline on day 7. The paper was **retracted in December 2021** (duplicate publication). It was never replicated and never described a lasting rise. | [V PubMed record, S for retraction reason], H |
| "Rebooting improves mental health" | Self-reports of benefit exist: Fernandez et al. 2021 analysed 104 abstinence journals and found perceived benefits and heavy difficulty. A preregistered survey (Prause and Binnie, *Sexualities* 2023) found more forum involvement associated with more distress after "relapse"; 28.9% of respondents reported feeling suicidal. Correlational, self-selected sample, and the first author is an open critic of the movement. | [V/S], M |

The field is also litigious: NoFap's founder sued a critic for defamation in 2019 (settled
2021), and NoFap LLC filed a new suit in late 2025 against researchers and the publisher Taylor
& Francis over a critical paper. [S (Retraction Watch, Feb 2026), H] A product that stays out of
the science fight, in both directions, is safer legally and clinically.

### 1.5 What a responsible product must not claim

Not in the app, the site, the store listing or ads:

1. That pornography use is an addiction in the medical sense, or that the user is "an addict".
2. Dopamine reset, detox, brain rewiring, neuroplastic healing, "reboot" or a fixed number of days
   after which the brain is "normal".
3. Any sexual-function benefit (erections, libido, performance) or hormone benefit (testosterone).
4. Energy, confidence, attractiveness, focus, hair, voice or "superpowers".
5. Any success, recovery or relapse-prevention rate, including "clinically proven". No trial of
   this product exists.
6. That masturbation is harmful or that sexual abstinence is the goal.
7. That the app treats, cures or diagnoses anything, or replaces therapy.
8. That blocking is unbreakable. It is friction, and saying otherwise sets up a failure.

What it may say: it blocks what the user chose to block, adds time between an urge and an action,
offers exercises drawn from approaches that are used in therapy for this problem, and points to
professional help.

---

## 2. Techniques with support

**Overall quality of the evidence.** Two recent syntheses set the ceiling for any claim:

- Roza et al., *Archives of Sexual Behavior* 2024: 28 studies, 500 participants, only **4
  randomised trials**; most studies low quality with significant risk of bias; **all
  interventions rated low or very low on GRADE**. [V (abstract), H]
- López-Pinar et al., *Journal of Behavioral Addictions* 2025 (meta-analysis): 20 studies, 2,021
  participants; psychotherapy beat controls on PPU, frequency and sexual compulsivity with large
  effects, small for craving; limited by high risk of bias. [V (abstract), H]
- A 2026 scoping review of CBT protocols (11 studies): all reported reductions, **none reported
  complete abstinence**, and self-guided formats had high dropout. [S, M]

So: second and third wave CBT (which includes ACT) is the most studied family, it looks helpful,
and nobody should promise a result.

| Technique | Best evidence found | Strength for this use | How the product uses it |
|---|---|---|---|
| **ACT** (acceptance and commitment therapy) | Crosby and Twohig, *Behavior Therapy* 2016, 47(3):355-366: 28 men, 12 individual sessions vs waitlist. Viewing fell **93% vs 21%**. At 3 months: 86% reduction, **35% had stopped**, **74% had reduced by 70% or more**. No improvement in quality of life. Sample almost entirely Latter-day Saints. Earlier uncontrolled study (n = 6): 85% reduction. [V] | Low-moderate (one small RCT, therapist-delivered, narrow sample) | Values sentence ("what do you want this for?"), "a thought is not an order", willingness to feel the urge without acting |
| **Group CBT** | Hallberg et al., *Journal of Sexual Medicine* 2019, 16(5):733: **137 men** with hypersexual disorder, 7 weeks of group CBT (n = 70) vs waitlist (n = 67); supported efficacy. Internet-delivered version: single-arm pilot, 2020. [V] | Low-moderate | Trigger mapping, if-then plans, lapse review |
| **Online self-help (MI + CBT + mindfulness)** | Bőthe et al., *Journal of Behavioral Addictions* 2021 ("Hands-off"): 264 randomised, six weekly modules vs waitlist; PPU lower at six weeks (d = 1.32). **Dropout 89.4% in the intervention arm**, overall follow-up 34.5%. [V] | Low (attrition) | Shows self-guided content can help those who stay, and that most people leave: keep interactions short |
| **Mindfulness-based relapse prevention / urge surfing** | Holas et al. 2020 (*J Behav Addict* 9(4):1088-1092): 13 men with CSBD, 8 weeks, less time on pornography, lower anxiety and depression; no control. Sniewski 2020: 12 single cases, twice-daily meditation, "encouraging but inconclusive". Outside sexuality: Bowen et al., *JAMA Psychiatry* 2014 (n = 286, substance use, 12-month benefit over usual care, no clear advantage over standard relapse prevention); Bowen and Marlatt 2009, brief urge-surfing instruction in student smokers: urges did not differ, smoking fell. [V] | Low for pornography, moderate by analogy | The 3-minute urge-surf timer. Honest framing: it does not make the urge smaller, it changes what you do with it |
| **If-then plans** (implementation intentions) | Gollwitzer and Sheeran 2006 (*Advances in Experimental Social Psychology* 38:69-119): d = 0.65 across **94 tests**. A later synthesis of 642 tests reports smaller effects (about d = 0.27 for behaviour, lower after bias correction). [V for 2006, S for the later figures] Not tested for pornography. | Moderate in general, untested here | One editable plan per trigger: "If I feel the urge when ____, then I will ____" |
| **Trigger awareness / HALT** | HALT (hungry, angry, lonely, tired) comes from mutual-aid tradition; no controlled test. [V] A small diary study (Henry et al., *Arch Sex Behav* 2025, n = 22) found the triggers people named most were loneliness, stress and social media use, yet measured loneliness did not rise before episodes. [S] | Low; useful as a prompt, not as a fact | A one-tap check-in. Never tell the user what their trigger "is"; show their own pattern back to them |
| **Stimulus control** | Standard component of CBT relapse prevention; no trial isolates it for pornography. [K] | Low direct, strong face validity | The DNS shield itself, plus user-added sites and "risky hours" |
| **Delay and distract** | Standard relapse-prevention component. [K] | Low direct | The disable delay; the "step away for 10 minutes" option |
| **Self-compassion after a lapse** (vs the abstinence violation effect) | Marlatt and Gordon's model: after a lapse, blaming a fixed flaw in oneself plus guilt turns a slip into a relapse. [K] Reid et al. 2014 (172 men): self-compassion partly offsets shame and rumination in hypersexuality. Phillips and Moen 2019 (n = 364): low self-compassion linked to more hypersexual behaviour. [S/V] All correlational. | Low-moderate | Lapse screen: no reset drama, a friend's voice, one question about what happened before, one small next step |
| **Commitment devices and friction** | Bryan, Karlan and Nelson, *Annual Review of Economics* 2010, 2:671-698 (hard vs soft commitments). Giné, Karlan and Zinman 2010: a voluntary deposit contract raised verified smoking cessation, still visible at 12 months; uptake was low. [V/K] | Moderate in general | A delay the user sets while calm. Soft, time-based commitment; no money at stake |
| **Accountability partners** | No randomised or controlled trial found. Outcome figures come from vendors (for example "89% eliminated", "95% chance") with no comparison group or no source. Qualitative studies (n = 27; n = 8) say participants value it. [S] | Very low, with real privacy and coercion risk (section 4) | Not in version 1 |
| **Streak counters** | Silverman and Barasch, *Journal of Consumer Research* 2023, 49(6):1095-1117 (seven studies): intact streaks raise engagement, **broken streaks lower it** (66% vs 58% in one study); the drop is larger when people blame themselves and smaller when the streak can be repaired. No study tests day counters for pornography. [V] | Mixed: a motivator that turns into a punishment exactly at the moment of a lapse | Optional and quiet, paired with a 30-day view that a lapse does not erase |

**Twelve-step groups.** Widely used and free. No controlled evidence was found in this search
for pornography specifically. They suit some people and not others, and the fellowships differ
in how they define sobriety (section 3).

---

## 3. Crisis and referral

### 3.1 When the product should suggest professional help

These are product heuristics drawn from the ICD-11 description, not clinical cut-offs. Thresholds
should be configurable constants and the suggestion should appear at most once a week.

| Signal (all local, all user-reported or counted on device) | Response |
|---|---|
| The user says, in onboarding or later, that this has lasted six months or more, that they have tried to cut down several times, or that it is hurting work, study, sleep, money or a relationship | Show the "this deserves a professional conversation" card once, with the resource list |
| Several lapses or many block-override attempts within 7 days (suggested starting values: 5 lapses, or 10 override attempts) | Professional-help suggestion (message `help.pattern`) |
| Two or more check-ins of "very bad" within 7 days | Same, with the crisis line visible |
| The user picks "talk to someone" in the panic flow, or anything indicating despair or self-harm | Crisis card immediately (CVV 188), above everything else |
| The user asks for help with something the app does not cover (substance use, relationship crisis, illegal content) | Say it is outside what UBI can do and show the general resource list |

The app does not monitor text or screens for suicidality. The crisis card is reachable in one
step from every nofap screen, always.

### 3.2 Resources checked in this session

"Checked" here means confirmed through search results that quote an official or academic source.
No resource site could be opened directly (section 0). **Re-verify every number and URL on the
live site before release**, and keep the list in one data file so it can be corrected without a
code change.

**Brazil**

| Resource | What it is | Contact | Status |
|---|---|---|---|
| **CVV, Centro de Valorização da Vida** | Volunteer emotional-support and suicide-prevention service, founded 1962, non-religious | Phone **188**, national, 24 hours, free from landline or mobile (cooperation agreement with the Ministry of Health); chat `cvv.org.br/chat`; e-mail via `cvv.org.br/e-mail` | [V] CVV's own quarterly reports, 2024-2025. H |
| **SAMU** | Public emergency medical service | **192** | [K], listed by the Ministry of Health among services that refer to CAPS. H |
| **CAPS, Centros de Atenção Psicossocial (SUS)** | Public community mental-health centres | Walk-in: "porta aberta", no appointment needed for the first visit; find the local unit through the city health department or `gov.br/saude/pt-br/caps` | [V] Ministry of Health page quoted in results. H |
| **Psychologist registered with the CFP** | Conselho Federal de Psicologia keeps a national register; online services are regulated (Resolução CFP 11/2018 created the e-Psi register; later rules include Resolução CFP 09/2024) | Check a professional's registration on the CFP site (`site.cfp.org.br`, "Cadastro Nacional de Profissionais de Psicologia"); e-Psi at `e-psi.cfp.org.br` | [S] Existence and public nature confirmed; whether e-Psi is still the required register after the 2024 resolution was **not** confirmed. M. Recommended wording: "procure um psicólogo com registro ativo no CRP" |
| **DASA, Dependentes de Amor e Sexo Anônimos** | The Brazilian fellowship linked to SLAA (Sex and Love Addicts Anonymous); twelve-step, free | Official Portuguese literature is hosted at `slaafws.org/lang/portuguese/`; a Brazilian site (`slaa.org.br`) could not be confirmed in this session | [V] that it exists and is the official SLAA affiliate. Meeting availability varies by region (one older report says the Minas Gerais group had closed). M. **Do not ship a DASA URL until someone opens it** |
| **AISEP, IPq-HCFMUSP (São Paulo)** | University outpatient clinic for excessive sexual drive (Ambulatório de Impulso Sexual Excessivo e Prevenção de Desfechos Negativos Associados ao Comportamento Sexual), Instituto de Psiquiatria | No intake contact found | [V] that it exists (academic papers). Do not list until an intake channel is confirmed. L for listing |

**International**

| Resource | What it is | Status |
|---|---|---|
| **Find A Helpline** (`findahelpline.com`, run by ThroughLine, New Zealand) | Directory of 1,300+ verified helplines in 130+ countries, human-staffed only | [V] H. Best single international fallback |
| **SAA, Sex Addicts Anonymous** (`saa-recovery.org`) | Twelve-step fellowship with a meeting finder covering in-person, phone and online meetings | [S] M. English-language; no Brazilian presence confirmed |
| **SLAA** (`slaafws.org`) | Parent fellowship of DASA, meeting finder | [V] M |
| **988 Suicide and Crisis Lifeline** (United States) | Crisis line | [S/K] H |

**Looked for and not confirmed, so not to be listed:** a Brazilian site for Sexaholics Anonymous
("Sexólicos Anônimos"); Brazilian SAA meetings; SMART Recovery meetings for sexual behaviour.
Note also [K, M] that Sexaholics Anonymous (`sa.org`) defines sobriety in terms of sex only
within marriage between a man and a woman, which makes it a poor default referral for a neutral
product.

**Open item:** people worried about illegal content need a different kind of service. None was
verified for Brazil in this session; leave it out of version 1 copy and research it separately.

---

## 4. Comparable apps: how they work and how they are criticised

| Product | Mechanism | Commitment / uninstall friction | Criticism and incidents |
|---|---|---|---|
| **Covenant Eyes** | "Screen Accountability": periodic screenshots, blurred on the device, analysed, report sent to a chosen "ally". About US$17-18 per month. On iOS, limited to its own browser. [S] | Uninstall alerts to the ally [K] | WIRED, September 2022 ("The Ungodly Surveillance of Anti-Porn 'Shameware' Apps"): Android accessibility permissions used to capture screens and record every site; a church network required staff to install it (up to 450 members enrolled). **Removed from Google Play on 21 September 2022, reinstated 1 March 2023** (company statement). [S, H] |
| **Accountable2You** | Monitoring and reporting only, **blocks nothing**: logs sites, searches, app use, user-defined trigger words; reports and instant alerts to partners. From US$9 per month. [V vendor docs] | Partner alerts | Same WIRED investigation; one user was confronted by a pastor for reading a Wikipedia article on atheism. Suspended on Google Play 21 September 2022, back 11 March 2023 (company statement). [S] |
| **BlockerX** (Atmana Tech) | Blocks through a local VPN profile; accountability partner; uninstall notification to the partner. 5.8M+ Android installs. [S] | Partner notified on uninstall | Billing and refund complaints (BBB, consumer sites), blocker switching itself off; App Store privacy label lists identifiers, usage data and diagnostics as **used to track** the user. [S, M] |
| **Brainbuddy** | Gamified daily programme with check-ins and streak visuals; marketing built on "rewire your brain" language. [K, L: not checked in this session] | Streaks | Makes exactly the neuroscience claims section 1.5 rules out |
| **Fortify** (Impact Collective, partner of Fight the New Drug) and **Relay** | Education modules, tracking, community; Relay centres on small accountability groups. [S] | Social | Outcome numbers on their pages ("over 90% report…", "95% chance…") have no stated method or source. [S] |
| **Quittr** | Viral quit-porn app with streaks and community | Streaks | 404 Media, March 2026: a Firebase misconfiguration exposed intimate data, including masturbation habits, of **more than 600,000 users**; a researcher warned the developer in September 2025; the company denied the problem for months. [S, H] |
| **Cold Turkey Blocker** (desktop) | Local blocker for sites and apps | "Locked" blocks cannot be ended early; the app cannot be uninstalled while a block is active; lock types include timer, password, random text; "Frozen Turkey" locks the whole computer. Stricter locks are paid. [S] | Users report bypasses (other OS accounts, killing the process). No surveillance: nothing leaves the machine |
| **Freedom** | Cross-device blocker with a sync account | **Locked Mode**: cannot change or quit during a session on Mac and Windows; **one early exit per 7 days**; an optional setting disables early exit permanently. [V vendor docs] | Needs an account and a cloud service |
| **Plucky / Pluckeye** | Self-control filter built around a **delay** | Raising the delay is instant; **lowering it takes effect only after the current delay has passed**; uninstall is allowed only when the delay is 0; docs suggest starting at 10 seconds; per-rule delays (for example 30 minutes for new allowances). [V vendor docs] | Steep learning curve. No reporting to anyone. The cleanest self-control model found |

**Store and platform policy** (relevant if the product ever goes to mobile; the desktop app is
distributed outside the stores)

- **Google Play, Accessibility API policy:** the `IsAccessibilityTool` flag is only for apps
  whose core purpose is supporting people with disabilities; monitoring apps are named as *not*
  qualifying and need prominent disclosure. [V]
- **Google Play, Stalkerware policy** (part of the malware policy): only apps "exclusively
  designed and marketed" for parents or enterprise may monitor another person, must declare the
  `isMonitoringTool` manifest flag, and "cannot be used to track others, such as a spouse, even
  with their knowledge and permission, regardless if persistent notification is displayed". [V]
  A self-control tool that reports to a partner sits uncomfortably close to this.
- **Apple App Review Guideline 5.4 (VPN apps):** VPN apps may not sell, use or disclose data to
  third parties; a content blocker that installs a local tunnel is likely to be reviewed as a VPN
  app; the Screen Time route needs the Family Controls entitlement. [S, M]

**Lessons**

1. The surveillance model (screenshots or logs sent to another person) is where the harm, the
   press and the store removals are. ubiqX already has a stronger position: the AI looks, a
   verdict is produced, and no image is kept or sent to a person.
2. The products with the least controversy (Cold Turkey, Plucky, Freedom) are the ones the user
   points at themselves, with time-based friction.
3. Intimate data on a server is the single largest risk. Local-only storage is a feature to
   state plainly.
4. Marketing claims in this category are mostly unsupported. A product that makes none stands
   out.

---

## 5. Naming: can the product be called "nofap"?

### 5.1 What was found

| Register | Record | Class | Status as reported |
|---|---|---|---|
| USPTO | NOFAP, Serial 86/001,394, **Reg. 4,591,410**, registered August 2014 | 41 (education) | Justia: Section 8 and 15 accepted; one aggregator marks it dead. [S, M] |
| USPTO | NOFAP, Serial 86/007,733, **Reg. 4,724,471**, registered April 2015 | 42 (computer and scientific services) | Registered, owner NoFap LLC. [S, M] |
| USPTO | NOFAP, Serial 87/129,763, **Reg. 5,160,831**, registered 14 March 2017 | 25 (clothing), 35 (advertising, business) | Registered; filed by Alexander Julian Rhodes (Pittsburgh, PA), later assigned to NoFap LLC. [S, M] |
| USPTO, portfolio | 7 filings under NoFap LLC: 3 registered, 4 cancelled | 25, 35, 41, 42, 45 | Aggregator count. [S, L] |
| UK IPO | "NoFap" application reported in Trade Marks Journal 2026/035, 28 August 2026 (two journal entries were returned; which one is NoFap was not confirmed) | 9 (software) | Applicant not shown in the result. [S, L] |
| India | NOFAP applications by NOFAP LLC, filed September 2026 | 41, 42 | "Ready for examination". [S, L] |
| **INPI (Brazil)**, EUIPO, WIPO Madrid | **Nothing found** | | Not searchable from this session. **Unknown, not "clear"**. |

NoFap's own site states that "NoFap®" and "Fapstronaut™" are trade and service marks of NoFap
LLC and restricts their use to describe or market other services. [S, M] The owner is an active
litigant: a 2019 defamation suit (settled 2021), a late-2025 suit against researchers and Taylor
& Francis preceded by a reported US$20 million demand, and a suit against Pornhub's owner whose
17 counts include trademark dilution. [S, H]

### 5.2 What that implies

- **Do not use "NoFap" in anything a user, a store or a search engine sees:** product name,
  window title, installer, bundle identifier, process or service name, DNS profile name, domain,
  site copy, store keywords, or "the NoFap app for…" comparisons. US Class 42 covers the kind of
  online service this product is. [M]
- **Brazil is not a safe harbour even if INPI has no record.** Under the Brazilian Industrial
  Property Law, registration can be refused for a mark the applicant evidently could not be
  unaware of when its holder is in a treaty country (art. 124, XXIII), and well-known marks get
  protection without local registration (art. 126). [K, M] The 2026 filings in the UK (Class 9)
  and India suggest the owner is extending coverage. Get a trademark search at INPI (classes 9,
  42, 44 and 45) and counsel's opinion before any name is fixed.
- **Internal code is a smaller risk** because it is not use in commerce, but identifiers leak
  (crate names, log lines, config keys, file paths, an open-source repository). The safe choice
  is a neutral internal identifier too, for example `shield` or `guard`. If the owner insists on
  `nofap` as the module name, keep it strictly out of every user-visible and published string and
  add a test that fails if "nofap" appears in the i18n bundles or the packaged metadata. [M]
- **There are also product reasons to avoid it.** "Fap" is English slang for masturbation; the
  term commits the product to a masturbation-abstinence stance it should not take (section 1.5),
  it is crude in a store listing, and it means little in Portuguese.

### 5.3 Eight alternative names

None has been cleared. Each needs an INPI and USPTO search and a domain check.

| # | Name (PT-BR / EN) | Idea | Notes |
|---|---|---|---|
| 1 | **Leme** / Leme | "Você no leme": the user steers, UBI holds the wheel steady | Short, agency without shame, works untranslated. Preferred |
| 2 | **Maré** / Maré | An urge is a tide: it rises and goes out | Ties directly to urge surfing. Also a Rio place name |
| 3 | **Trégua** / Truce | A truce with yourself, not a war | Non-combative, fits self-compassion |
| 4 | **Respiro** / Breather | The pause between urge and action | Describes the mechanism (delay) |
| 5 | **Farol** / Lighthouse | A light that warns, does not judge | Common word, expect collisions |
| 6 | **Firme** / Steady | "Fica firme, eu tô aqui" | Closest to UBI's voice |
| 7 | **Clareza** / Clarity | A clear head, a clear screen | Generic, harder to protect |
| 8 | **Guarda do UBI** / UBI Guard | The mascot standing at the door | Best for the module inside ubiqX: "UBI Guard" as a feature name |

Suggested pairing: **"UBI Guard / Guarda do UBI"** for the module inside ubiqX, and **"Leme, by
ubiqX"** (or "Maré") for the standalone product. Store subtitle, descriptive and claim-free:
"Bloqueio de pornografia e apoio para mudar o hábito" / "Pornography blocking and support for
changing the habit".

---

## 6. Tone guide for UBI

UBI already speaks in short, warm, first-person lines ("Eu cuido do registro — segue o jogo").
The nofap voice is the same mascot on a more delicate subject.

**Principles**

1. **The user decided; UBI keeps the agreement.** Every block refers back to the user's own
   choice ("a lista que você montou", "como combinamos"). UBI is never the authority.
2. **No shame, no praise inflation.** A lapse gets a calm voice. A milestone gets a nod, not
   fireworks. Self-worth is never tied to a number.
3. **Tentative about what it sees.** Vision can be wrong. "Parece que…", "Posso estar
   enganado…", and always a one-tap "that was not it".
4. **Short.** One or two sentences, one action. Dropout in self-guided programmes is very high;
   long text is not read.
5. **Describe, do not explain the brain.** No neuroscience, no percentages, no "studies show".
6. **Discreet by default.** The screen may be visible to someone else. The default block page
   says "Página bloqueada pelo UBI" and nothing about why. The supportive text appears when the
   user opens the app or chooses "detailed messages".
7. **Neutral about the person.** No assumptions about gender, partner, religion or orientation.
   Gender-neutral Portuguese where possible ("você", verbs, no "bem-vindo/bem-vinda").
8. **Honest about limits.** "Sou um mascote, não um terapeuta."
9. **Always an exit to a human.** The help card is one step away on every screen.

**Never say** (Portuguese examples to keep out of every string and out of any AI prompt)

| Avoid | Why |
|---|---|
| "viciado", "dependente", "seu vício" | Diagnosis the app cannot make |
| "recaída" as a label on the user; "você falhou", "perdeu tudo", "voltou à estaca zero" | Feeds the abstinence violation effect |
| "limpo", "sujo", "puro", "pecado", "vergonha" | Moral framing |
| "homem de verdade", "seja forte", "força de vontade" | Shaming, gendered, and wrong about how urges work |
| "seu cérebro está se recuperando", "dopamina", "reprogramar", "reboot", "detox" | Unsupported claims |
| "X% das pessoas conseguem", "comprovado" | No data for this product |
| "sua parceira", "sua esposa", "pense na sua família" | Assumptions and guilt |
| "nunca mais", "para sempre" | All-or-nothing goals make lapses heavier |

Preferred vocabulary: "vontade" (urge), "deslize" (slip), "combinado" (what you agreed with
yourself), "praticar", "do jeito que você escolheu".

**If AI-generated text is used at all.** Version 1 should use only the reviewed message bank;
the vision model returns a classification, never prose shown to the user. If free-form coaching
is added later, the system prompt must embed the "never say" list, forbid diagnosis and medical
claims, cap length, and hand over to the crisis card on any sign of despair.

---

## 7. Message bank: 40 messages, PT-BR with English equivalents

Keys are suggestions. `{reason}` and `{days}` are user-supplied or computed locally. Messages
1-6 show on a block; only `block.discreet` is used when discreet mode is on.

### 7.1 Blocked-site intercept

| # | Key | PT-BR | English |
|---|---|---|---|
| 1 | `block.default` | Esse endereço está na lista que você montou. Segurei a porta pra você. | That address is on the list you set up. I held the door for you. |
| 2 | `block.breathe` | Bloqueado, como combinamos. Respira comigo por um minuto? | Blocked, as we agreed. Breathe with me for a minute? |
| 3 | `block.choice` | Você decidiu isso num momento de calma. Vale confiar naquela decisão agora. | You decided this in a calm moment. It's worth trusting that decision now. |
| 4 | `block.next` | A vontade veio. Tudo bem, ela passa. O que você ia fazer antes disso? | The urge showed up. That's okay, it passes. What were you about to do before this? |
| 5 | `block.discreet` | Página bloqueada pelo UBI. | Page blocked by UBI. |
| 6 | `block.repeat` | Terceira tentativa em pouco tempo. Parece um momento difícil. Quer fazer a onda de 3 minutos comigo? | Third try in a short while. Looks like a tough moment. Want to ride the 3-minute wave with me? |

### 7.2 Vision-detected content

| # | Key | PT-BR | English |
|---|---|---|---|
| 7 | `vision.soft` | Posso estar enganado, mas o que está na tela parece fugir do seu combinado. Quer que eu feche? | I might be wrong, but what's on screen looks outside your plan. Want me to close it? |
| 8 | `vision.closed` | Fechei a janela, como você pediu que eu fizesse. Nenhuma imagem foi guardada. | I closed the window, as you asked me to. No image was kept. |
| 9 | `vision.added` | Coloquei esse site na sua lista pra ele não te pegar de surpresa de novo. | I added that site to your list so it won't catch you off guard again. |
| 10 | `vision.wrong` | Errei? Me avisa aqui e eu ajusto. Sem drama. | Did I get it wrong? Tell me here and I'll adjust. No fuss. |
| 11 | `vision.after` | Passou. Sem sermão. Quer dois minutos pra respirar ou prefere seguir o dia? | It's over. No lecture. Want two minutes to breathe, or would you rather get on with your day? |

### 7.3 Urge-surf timer (3 minutes, one step per screen)

| # | Key | PT-BR | English |
|---|---|---|---|
| 12 | `surf.start` | Vamos surfar essa vontade juntos. São 3 minutos. Você não precisa lutar contra ela, só observar. | Let's surf this urge together. Three minutes. You don't have to fight it, only watch it. |
| 13 | `surf.body` | Onde você sente a vontade no corpo? Peito, barriga, mãos? Só repare. | Where do you feel the urge in your body? Chest, stomach, hands? Just notice. |
| 14 | `surf.breath` | Inspira contando até 4… solta contando até 6. Deixa o ar fazer o trabalho. | Breathe in for 4… out for 6. Let the breath do the work. |
| 15 | `surf.wave` | Vontade é onda: sobe, chega no pico e desce. Você está em cima da prancha. | An urge is a wave: it rises, peaks and falls. You're on the board. |
| 16 | `surf.thought` | Se vier um pensamento tipo "só dessa vez", repare nele e deixe passar. Pensamento não é ordem. | If a thought like "just this once" shows up, notice it and let it go by. A thought is not an order. |
| 17 | `surf.rate` | De 0 a 10, como está a vontade agora? | From 0 to 10, how strong is the urge now? |
| 18 | `surf.end` | Pronto. Você ficou com a vontade sem obedecer. Isso é treino, e conta. | Done. You stayed with the urge without obeying it. That is practice, and it counts. |

### 7.4 After a lapse

| # | Key | PT-BR | English |
|---|---|---|---|
| 19 | `lapse.first` | Aconteceu. Um deslize não apaga o que você já construiu. | It happened. One slip doesn't erase what you've built. |
| 20 | `lapse.kind` | Fale com você como falaria com um amigo na mesma situação. | Talk to yourself the way you'd talk to a friend in the same spot. |
| 21 | `lapse.learn` | Sem culpa, só curiosidade: o que estava acontecendo antes? Cansaço, tédio, estresse, solidão? | No blame, only curiosity: what was going on beforehand? Tiredness, boredom, stress, loneliness? |
| 22 | `lapse.plan` | Quer ajustar um plano "se… então…" pra próxima vez que esse gatilho aparecer? | Want to adjust an "if… then…" plan for the next time that trigger shows up? |
| 23 | `lapse.resume` | O próximo passo é pequeno: fechar a aba, beber água, voltar pro seu dia. Eu sigo aqui. | The next step is small: close the tab, drink some water, get back to your day. I'm still here. |

### 7.5 Streak milestones (low-key)

| # | Key | PT-BR | English |
|---|---|---|---|
| 24 | `streak.day1` | Um dia do jeito que você escolheu. Bom começo. | One day the way you chose. Good start. |
| 25 | `streak.week` | Uma semana. Sem fogos, só um aceno: você está praticando. | One week. No fireworks, just a nod: you're practising. |
| 26 | `streak.month` | Trinta dias. O que mudou na sua rotina que vale manter? | Thirty days. What changed in your routine that's worth keeping? |
| 27 | `streak.after_reset` | A contagem recomeça, a prática não. Nos últimos 30 dias, {days} foram do jeito que você queria. | The count restarts; the practice doesn't. In the last 30 days, {days} went the way you wanted. |

### 7.6 Check-in prompts

| # | Key | PT-BR | English |
|---|---|---|---|
| 28 | `checkin.daily` | Como você está hoje? Um toque basta. | How are you today? One tap is enough. |
| 29 | `checkin.halt` | Checagem rápida: fome, raiva, solidão ou cansaço? Qualquer um deles deixa tudo mais difícil. | Quick check: hungry, angry, lonely or tired? Any of them makes everything harder. |
| 30 | `checkin.evening` | Pelo seu histórico, o fim da noite costuma ser a hora mais difícil. Quer deixar um plano pronto pra hoje? | Going by your history, late evening tends to be the hardest time. Want to set a plan for tonight? |
| 31 | `checkin.ifthen` | Complete: "Se eu sentir vontade quando ____, então eu vou ____." | Fill in: "If I feel the urge when ____, then I will ____." |
| 32 | `checkin.values` | Pra que você quer isso? Escreva uma frase. Eu te lembro dela quando apertar. | What do you want this for? Write one sentence. I'll remind you of it when things get tough. |

### 7.7 Panic-button flow

| # | Key | PT-BR | English |
|---|---|---|---|
| 33 | `panic.open` | Estou aqui. Você fez bem em apertar. | I'm here. You did well to press this. |
| 34 | `panic.options` | O que ajuda agora: surfar a vontade (3 min), sair da tela (10 min) ou falar com alguém? | What helps right now: surf the urge (3 min), step away from the screen (10 min), or talk to someone? |
| 35 | `panic.away` | Levanta, bebe água, vai até a janela. Eu seguro tudo por aqui por 10 minutos. | Get up, drink some water, go to the window. I'll hold everything here for 10 minutes. |
| 36 | `panic.reason` | Você escreveu: "{reason}". Continua valendo? | You wrote: "{reason}". Still true? |
| 37 | `panic.crisis` | Se o que você sente é maior do que vontade (desespero, ou pensamentos de se machucar), ligue 188. É o CVV: grátis, 24 horas. Também há chat em cvv.org.br. Em emergência, ligue 192. | If what you feel is bigger than an urge (despair, or thoughts of hurting yourself), please reach a crisis line now. In Brazil, call 188 (CVV, free, 24 hours). Elsewhere, see findahelpline.com. In an emergency, call your local emergency number. |

### 7.8 Professional-help suggestion

| # | Key | PT-BR | English |
|---|---|---|---|
| 38 | `help.pattern` | Tem sido difícil várias vezes nesta semana. Isso não é falta de caráter, e você não precisa resolver sozinho. Um psicólogo pode ajudar de verdade. | It's been hard several times this week. That's not a character flaw, and you don't have to sort it out alone. A psychologist can really help. |
| 39 | `help.how` | Quer ver como encontrar atendimento? Há psicólogos com registro no CRP, o CAPS pelo SUS e grupos de apoio mútuo. | Want to see how to find support? There are licensed therapists, public mental-health services and mutual-help groups. |
| 40 | `help.limits` | Eu sou um mascote, não um terapeuta. Posso bloquear, lembrar e acompanhar. Cuidar da causa é trabalho pra fazer com um profissional. | I'm a mascot, not a therapist. I can block, remind and keep you company. Working on the cause is something to do with a professional. |

Copy notes: message 15 is a metaphor, not a duration claim, so no "it passes in N minutes".
Message 30 must only appear when the user's own local log supports it. Message 37 is the only
place a phone number appears; it comes from the resource data file, not a hard-coded string.

---

## 8. Feature set

### 8.1 Version 1 (the smallest set that is honest and useful)

| # | Feature | Grounded in |
|---|---|---|
| 1 | **Explicit opt-in onboarding**: what is blocked, what the AI looks at, what is sent to which provider, what is stored (counts, no images), how to turn it off. Adults only, on your own computer | Privacy framing; store-policy lessons |
| 2 | **DNS shield** with the extensive list plus user-added sites | Stimulus control |
| 3 | **Vision check on unknown sites and apps** with the existing AI: verdict only, no image retained, tentative message, one-tap "that was not it", optional auto-add of the domain to the local list | Section 4 lesson 1 |
| 4 | **Discreet block page** by default, detailed supportive text opt-in | Tone principle 6 |
| 5 | **Urge-surf timer** (3 minutes, messages 12-18) reachable from the block page, the tray and the panic button | MBRP / ACT |
| 6 | **Panic button**: surf, step away 10 minutes, or talk to someone (crisis card) | Delay and distract; crisis path |
| 7 | **Disable delay**: turning the shield off or removing a site takes effect after a user-chosen delay (suggested default 10 minutes, range 0 to 24 hours; raising is instant, lowering waits out the current delay). Urge-surf is offered during the wait. A separate, undelayed "restore my network" action exists for breakage and is logged locally | Commitment devices; Plucky and Freedom models |
| 8 | **Lapse flow**: "it happened" in one tap, self-compassion message, one optional trigger question, one optional if-then plan | Abstinence violation effect; implementation intentions |
| 9 | **One values sentence and up to three if-then plans**, shown back at block time | ACT; implementation intentions |
| 10 | **Quiet progress view**: days the way you chose in the last 30, plus an optional current streak, off by default | Silverman and Barasch 2023 |
| 11 | **Help page** with the verified resource list from a data file, and the "when to see a professional" description | Section 3 |
| 12 | **Local-only data**: counts and timestamps of blocks, surf sessions, lapses and check-ins; no domains of blocked attempts beyond what the user chose to add; one-click erase | Quittr lesson |
| 13 | **No claims**: marketing and in-app copy reviewed against section 1.5 | Section 1 |

For the standalone product the same set applies without the productivity tracker; if no AI
provider is configured, feature 3 is off and the app says so plainly.

### 8.2 Defer

| Feature | Why later |
|---|---|
| Accountability partner | No controlled evidence, real coercion and privacy risk, store-policy exposure. If built: user-initiated, weekly summary of counts only, revocable, never screenshots or URLs, never a way to install on another person's device |
| Community, forums, leaderboards, sharing streaks | Forum involvement is associated with more distress after lapses; moderation burden |
| Free-form AI chat counsellor | Needs safety design, crisis hand-off and evaluation first |
| Multi-week structured programme (ACT/CBT modules) | Worth doing with a clinical psychologist as author; very high dropout in self-guided programmes argues for short units |
| Validated self-screening questionnaire (for example the Brief Pornography Screen or PPCS-6) | Useful for the "see a professional" prompt, but licence terms for commercial use and Portuguese validation must be checked first |
| Hard, unbreakable locks (password held by someone else, whole-computer lock) | Support burden, lockout risk, and they invite a control relationship |
| Mobile apps | Different enforcement and store policy (section 4) |
| Any cloud sync of nofap data | Only with end-to-end encryption and a clear reason |
| Therapist directory or referral partnerships | Needs vetting and CFP advertising rules |
| Content for partners and families, minors | Different product, different ethics |

---

## 9. Sources

Peer-reviewed and clinical

- WHO ICD-11, 6C72 Compulsive sexual behaviour disorder: https://icd.who.int/ (entity 1630268048); mirror https://www.findacode.com/icd-11/code-1630268048.html
- Review reproducing the ICD-11 requirements and moral-distress exclusion (Frontiers in Psychiatry, 2025): https://www.frontiersin.org/articles/10.3389/fpsyt.2025.1621136/full
- Grubbs, Perry, Wilt, Reid (2019), Arch Sex Behav 48(2):397-415: https://pubmed.ncbi.nlm.nih.gov/30076491/
- Bőthe et al. (2024), Addiction 119(5):928-950: https://researchonline.ljmu.ac.uk/id/eprint/22192/
- Bőthe et al. (2023), CSBD in 42 countries (press summary): https://www.psypost.org/almost-5-of-people-may-experience-compulsive-sexual-behavior-disorder-worldwide/
- Crosby and Twohig (2016), Behav Ther 47(3):355-366: https://digitalcommons.usu.edu/psych_facpub/1355
- Hallberg et al. (2019), J Sex Med 16(5):733: https://pubmed.ncbi.nlm.nih.gov/30956109/ ; internet CBT pilot (2020): https://pubmed.ncbi.nlm.nih.gov/32900671/
- Bőthe et al. (2021), "Hands-off" online intervention: https://pmc.ncbi.nlm.nih.gov/articles/PMC8987418
- Roza et al. (2024), systematic review of treatments: https://www.pubmed.gov/37880509/
- López-Pinar et al. (2025), meta-analysis: https://pubmed.ncbi.nlm.nih.gov/40126561/
- CBT protocols scoping review (2026): https://culturereframed.org/library-article/cognitive-behavioral-therapy-based-interventions-for-problematic-pornography-use-a-scoping-review/
- Holas et al. (2020), MBRP pilot for CSBD: https://pubmed.ncbi.nlm.nih.gov/33216012/
- Sniewski (2020), meditation single-case series: https://openrepository.aut.ac.nz/handle/10292/13170
- Bowen et al. (2014), JAMA Psychiatry: https://www.psychiatry.wisc.edu/wp-content/uploads/2025/05/Article-1_Bowen-et-al-JAMA-2014.pdf
- Bowen and Marlatt (2009), "Surfing the urge": https://pubmed.ncbi.nlm.nih.gov/20025372/
- Gollwitzer and Sheeran (2006): https://kops.uni-konstanz.de/entities/publication/2e749bfb-8533-437c-8203-7e788c910c5f
- HALT review (2026): https://www.ncbi.nlm.nih.gov/pmc/articles/PMC12827198/
- Henry et al. (2025), diary study: https://link.springer.com/article/10.1007/s10508-025-03287-z
- Phillips and Moen (2019), self-compassion and hypersexual behaviour: https://self-compassion.org/wp-content/uploads/2020/10/PhillipsMoen2019.pdf
- Reid et al. (2014), secondary summary: https://www.recoveryranch.com/addiction-blog/self-compassion-can-ease-shame-associated-with-sex-addiction/
- Bryan, Karlan, Nelson (2010): https://econpapers.repec.org/RePEc:anr:reveco:v:2:y:2010:p:671-698
- Giné, Karlan, Zinman (2010): https://es.poverty-action.org/sites/default/files/publications/put_your_money_where_your_butt_is.pdf
- Silverman and Barasch (2023), J Consumer Research 49(6):1095-1117: https://udspace.udel.edu/items/42ce576b-8e1f-429a-8541-e29e48dbcfbb
- Fernandez, Kuss, Griffiths (2020), short-term abstinence review: https://irep.ntu.ac.uk/id/eprint/39178
- Fernandez, Kuss, Griffiths (2021), rebooting journals: https://link.springer.com/article/10.1007/s10508-020-01858-w
- Seven-day abstinence study (press summary): https://www.psypost.org/regular-pornography-users-do-not-experience-withdrawal-symptoms-over-7-day-abstinence/
- Prause and Binnie (2023), Sexualities: https://journals.sagepub.com/doi/abs/10.1177/13634607231157070 ; https://openresearch.lsbu.ac.uk/item/93513
- Grubbs and Gola (2019), J Sex Med: https://read.qxmd.com/read/30621919/is-pornography-use-related-to-erectile-functioning-results-from-cross-sectional-and-latent-growth-curve-analyses
- Dwulit and Rzymski (2019), J Clin Med: https://pubmed.ncbi.nlm.nih.gov/31247949/
- Park et al. (2016), Behav Sci: https://pmc.ncbi.nlm.nih.gov/articles/PMC5039517
- Jiang et al. (2003), retracted: https://pubmed.ncbi.nlm.nih.gov/12659241/ ; https://www.snopes.com/fact-check/increase-testosterone-45-percent/
- Ley, Prause, Finn (2014), press release: https://www.sciencedaily.com/releases/2014/02/140212153252.htm

Resources

- CVV quarterly report, third quarter 2025: https://cvv.org.br/wp-content/uploads/2025/11/RelatorioCVV_3Tri_2025.pdf
- CVV free calls (CONASS): https://www.conass.org.br/prevencao-ao-suicidio-conta-com-ligacao-gratuita-em-todo-pais/
- CAPS, Ministry of Health: https://www.gov.br/saude/pt-br/caps
- CFP: https://site.cfp.org.br/
- SLAA literature in Portuguese (DASA): https://slaafws.org/lang/portuguese/Preambulo.pdf
- AISEP, IPq-HCFMUSP (academic paper): https://pmc.ncbi.nlm.nih.gov/articles/PMC6899385
- Find A Helpline: https://findahelpline.com/about
- SAA: https://saa-recovery.org/

Comparable apps and policy

- WIRED investigation, secondary coverage: https://churchleaders.com/news/435095-covenant-eyes-responds-to-accusations-shameware.html ; in Portuguese: https://www.tecmundo.com.br/seguranca/248539-apps-bloquear-pornografia-espionavam-fieis-igrejas.htm
- Covenant Eyes response: https://www.christianpost.com/news/covenant-eyes-privacy-concerns-warns-against-spying.html
- Accountable2You statement: https://accountable2you.com/?p=14678 ; reports documentation: https://support.accountable2you.com/article/844-understand-accountable2you-reports
- Accountability software overview: https://en.wikipedia.org/wiki/Accountability_software
- BlockerX complaints: https://prod.bbb.org/us/ca/san-francisco/profile/computer-software/blockerx-atmana-1116-928483/complaints
- Fortify affiliate page: https://www.joinfortify.com/affiliates/ftnd ; Relay: https://joinrelay.app/blog/porn-addiction-accountability-partner
- Quittr leak (404 Media): https://404media.co/viral-quittr-porn-addiction-app-exposed-the-masturbation-habits-of-hundreds-of-thousands-of-users ; https://404media.co/multiple-hackers-warned-anti-porn-app-quittr-about-security-issue-for-months
- Cold Turkey: https://getcoldturkey.com
- Freedom Locked Mode: https://support.freedom.to/en/articles/1802927-locked-mode
- Plucky delay: https://docs.pluckeye.net/delay
- Google Play Accessibility API policy: https://support.google.com/googleplay/android-developer/answer/10964491
- Google Play monitoring and stalkerware requirements: https://support.google.com/googleplay/android-developer/answer/12955211
- Apple Guideline 5.4 discussion: https://developer.apple.com/forums/thread/844103

Trademark

- USPTO records via Justia: https://trademarks.justia.com/860/01/nofap-86001394.html ; https://trademark.justia.com/860/07/nofap-86007733.html ; https://trademark.justia.com/871/29/nofap-87129763.html
- Portfolio aggregator: https://trademarkdir.com/owner/nofap-llc
- UK Trade Marks Journal 2026/035: https://www.ipo.gov.uk/t-tmj/tm-journals/2026-035/UK00004358880.html
- India listings: https://www.registerkaro.in/trademark-search/nofap-8022906
- NoFap site notice: https://nofap.com/?p=859
- Litigation (Retraction Watch, February 2026): https://retractionwatch.com/2026/02/19/porn-addiction-recovery-group-nofap-lawsuit-taylor-francis-ucla

Related report in this folder: `research-dns-enforcement.md` (resolver addresses, per-OS DNS
enforcement).
