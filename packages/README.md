# Pale Packages

An agent often starts a session without the method its work needs. Pale Packages hold working methods that several projects can use, as a library whose versions a workspace pins by release content. A workspace selects the methods its people and agents need. Today people and agents open the pages directly; handing an agent the method for the duty it is doing, through the compiled profile its hooks and MCP server read, is the problem in front. Start with the project that owns the work. Find its instructions, files, tools and criteria for a good result. Then open the family below that helps with the decision in front of you.

| If you need to… | Start here |
| --- | --- |
| Make or review a code change, diagnose a failure, or run a repository check | [Engineering](engineering/INDEX.md) |
| Find who can decide or act, and what a tool is allowed to do | [Governance](governance/INDEX.md) |
| Choose a Git workflow, or design a contract, data model, service, recovery path or release | [Architecture](architecture/INDEX.md) |
| Write agent instructions, pass work to a specialist, or review a workflow | [Agent Work](agent-work/INDEX.md) |
| Build and compare workflows, handle retries, or bring in human review | [Automation](automation/INDEX.md) |
| Choose examples, trace a result, or find why a check stopped telling the truth | [Evaluation](evaluation/INDEX.md) |
| Plan research, work in notebooks, or judge what a source supports | [Research](research/INDEX.md) |
| Compare models, check numerical results, or estimate runtime cost | [Model Work](model-work/INDEX.md) |
| Choose a design direction, build an interface, or check how it looks and feels | [Design](design/INDEX.md) |
| Shape an article or guide, prepare a publication, or check a local render | [Publication](publication/INDEX.md) |
| Turn a user need into a capability and decide what to build first | [Product](product/INDEX.md) |
| Assess a business decision, operating plan or launch | [Business](business/INDEX.md) |
| Review a risk, control or claim before relying on it | [Assurance](assurance/INDEX.md) |
| Frame a money question, project what fails first, or close an owner's books for review | [Finance](finance/INDEX.md) |

## Use the pages directly

Open the family index, then the entry for your task. Read what it needs you to read first. Follow a related page when it helps with a distinct part of the work. Use the guide for a library only when the project uses that library or you are deciding whether to add it. A Markdown page explains how to work. The project's own configuration and tools tell you what actually runs.

For example, when fixing a page transition, start with the project's design references and motion tokens. Use Design to decide what should move and how the transition should respond to a second input. Open the guide for the library already in the project if you need its API. Check the transition in the rendered interface. For a Gazette build, use Publication to decide which render and checks you need, then use Gazette to run them. The publisher decides whether the result can go live.

For repository work, use Engineering's verification method and the repository's own check command. For local automation exports, use the [shared n8n and Appsmith tools](../tools/automation-verification/README.md). The project supplies the inputs and decides what differences are acceptable.

## Use the declarations in Pale

Each family has a `package.toml` with its resource IDs, source files and dependencies. A workspace chooses the resources it needs. The compiler reads their TOML relationships, includes required guidance and keeps related guidance available for later reading. The workspace supplies its own tool bindings. Project facts, design assets, credentials and operating state stay with their owners.

To install repository guidance before resident activation, use the [repository-guidance example](../docs/examples/guidance-workspace/README.md). To change a family, start with [Package authoring](../docs/package-authoring.md). Edit the source and its declarations together, then regenerate the index. See [source bundles](../docs/package-source-bundles.md) for copying a family and its licences to another checkout.

Engineering's review method adapts the finding checks and false-positive examples from [ECC](https://github.com/affaan-m/ECC/blob/591ab5cbd3f2f65860ea91c226e410b1502c8e2e/agents/code-reviewer.md). Source credits and their terms travel in [NOTICE](NOTICE).

Engineering, Agent Work, Architecture and Automation adapt practices from [tstack](https://github.com/t3dotgg/tstack/tree/11e37a74d0a6868c121cd0910ab2b3bd9873b92a); their source terms are in [NOTICE](NOTICE). The adaptations use the consumer's providers, permissions and repository rules:

| Upstream skill | Home in Pale and adaptation |
| --- | --- |
| `council` | [Review](engineering/review.md) carries independent review across the consumer's selected providers, findings and disagreements. It does not mandate three providers or numerical scores. |
| `sol-loop` | [Review](engineering/review.md) carries a fixed candidate, bounded rounds and follow-up on changed behaviour and unresolved findings. The consumer selects the reviewer; a missing required provider remains an open review. |
| `takeover` | [Handoff](engineering/handoff.md) and [repository strategy](architecture/repo/strategy.md) recover intent, current work and contributors' credit before continuing. They do not require a replacement PR when an in-place continuation fits. |
| `modernize` | [Repository strategy](architecture/repo/strategy.md) reassesses whether the change still belongs, then compares and checks the updated series. Rebasing, pushing and closing retain their own permissions. |
| `file-pr` | [Repository strategy](architecture/repo/strategy.md) checks for an existing request and leads its description with the problem and resulting behaviour. Draft status and attribution follow the repository. |
| `leaving-pr-comments` | [Review comments](architecture/repo/strategy.md#write-a-review-comment-that-answers-the-finding) answer the finding, preview the proposed post, use verified attribution and read back the result. No fixed bot label is imposed. |
| `babysit-pr` | [Repository strategy](architecture/repo/strategy.md) and [agent operation](agent-work/operate.md) use one observer through checks, reviews and the agreed outcome. They retain unresolved older findings and use the repository's actual review requirements, not a fixed bot list or bundled watcher. |
| `full-send` | [Unattended work](agent-work/unattended.md) and [workflow completion](automation/workflow.md) carry authorised work through its observed result. The phrase itself grants no push, merge, test or deployment permission. |
| `full-stop` | [Unattended work](agent-work/unattended.md) and [repository strategy](architecture/repo/strategy.md) stop at the agreed handoff and check for automatic merging. Ordinary emphasis does not select a workflow or change permission. |
