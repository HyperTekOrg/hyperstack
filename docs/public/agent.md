# Hyperstack Agent Setup

You are setting up Hyperstack, a real-time Solana data streaming platform. Follow these steps in order.

## Step 1: Install the Hyperstack CLI

Preferred (Cargo):
```
cargo install hyperstack-cli
```

If Cargo is not available, fall back to npm:
```
npm install -g hyperstack-cli
```

Verify installation:
```
hs --version
```

## Step 2: Install Agent Skills

The `npx skills` tool installs skills for 40+ coding agents including Cursor, Claude Code, OpenCode, Codex, Windsurf, VS Code Copilot, Cline, Roo Code, and many more. It auto-detects which agents are present and installs to the correct location for each.

```
npx skills add usehyperstack/skills
```

This installs context files that teach you (the agent) how to use Hyperstack's SDKs, CLI, and Rust DSL.

If you need to target a specific agent, or want to understand the options available, see the full `npx skills` documentation: https://github.com/vercel-labs/skills

Common options:
- `npx skills add usehyperstack/skills --all` installs all skills to all detected agents without prompts
- `npx skills add usehyperstack/skills -a claude-code` targets a specific agent
- `npx skills add usehyperstack/skills -a cursor -a opencode` targets multiple agents
- `npx skills add usehyperstack/skills -g` installs globally (available across all projects)

## Step 3: Discover Available Data

```
hs explore --json
```

This lists all available stacks you can stream data from. For each stack, get the full schema:

```
hs explore <stack-name> --json
```

For field-level detail on a specific entity:

```
hs explore <stack-name> <EntityName> --json
```

## Step 4: You Are Ready

You now have everything needed to build with Hyperstack.

Key rules:
- ALWAYS run `hs explore <stack> --json` before writing any Hyperstack code. Never guess entity names, field paths, or types.
- Use `hs explore <stack> <Entity> --json` to get exact field names, types, and view definitions.
- The primary public stack is `ore` (ORE mining data). Run `hs explore ore --json` to see its entities.
- For React apps: install `hyperstack-react` and `hyperstack-stacks`
- For TypeScript apps: install `hyperstack-typescript` and `hyperstack-stacks`
- To scaffold a new project quickly: `npx hyperstack-cli create my-app`

Full documentation: https://docs.usehyperstack.com
Agent skills reference: https://docs.usehyperstack.com/agent-skills/overview/
Prompt cookbook: https://docs.usehyperstack.com/agent-skills/prompts/
