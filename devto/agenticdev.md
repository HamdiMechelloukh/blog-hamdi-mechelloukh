---
title: "AgenticDev: a multi-LLM framework for generating tested code"
published: false
description: "How I designed a pipeline of 4 specialized LLM agents with LangGraph, prompt caching and automatic fix loop."
tags: llm, ai, python, langchain
canonical_url: https://www.hamdimechelloukh.com/blog/multi-llm-framework-agenticdev
---

In late 2025, after spending hours prompting LLMs one by one to generate code, a question kept nagging me: **what if multiple LLM agents could collaborate to produce a complete project?** Not a single agent doing everything, but a specialized team (an architect, a developer, a tester), each with its own role, tools, and constraints.

That's how **AgenticDev** was born, a Python framework that orchestrates 4 LLM agents to turn a plain-text request into tested, documented code.

In this article, I share the architecture decisions, the problems I ran into, and the lessons learned.

## Starting point: testing the limits of multi-agent collaboration

My initial goal was simple: explore how far LLM agents can collaborate autonomously. Not a throwaway POC, but a real pipeline where each agent has a clear responsibility:

- **Architect** — analyzes the request and produces a technical specification (`spec.md`)
- **Designer** — generates SVG assets from the spec
- **Developer** — implements the code following the spec and integrating the assets
- **Tester** — writes and runs tests, then sends failures back to the Developer

The idea is the **Agent as Tool** pattern: each agent is a node in an execution graph, not an LLM calling other LLMs chaotically.

## Architecture: why LangGraph over an LLM orchestrator

My first approach was letting an orchestrator agent (Gemini) dynamically decide which sub-agent to call, via function calls. It worked, but I quickly identified a problem: **the more generic the system, the more unpredictable it became.**

The LLM orchestrator could decide to skip the Designer, call the Tester before the Developer, or loop indefinitely. For a framework that needs to produce reliable code, that's a deal-breaker.

So I chose to **delegate orchestration to LangGraph**, a deterministic graph framework. The pipeline becomes explicit:

```
Architect → Designer → Developer → Tester
                                      │
                                      ▼ (tests fail?)
                                   Developer ← fix loop (max 3×)
```

Each node is an autonomous agent, but **execution order and retry logic are deterministic**. The LLM controls the *what* (generated content), but not the *when* (execution flow).

```python
_builder = StateGraph(PipelineState)
_builder.add_edge(START, "architect")
_builder.add_edge("architect", "designer")
_builder.add_edge("designer", "developer")
_builder.add_edge("developer", "tester")
_builder.add_conditional_edges(
    "tester",
    should_fix_or_end,
    {"fix": "fix_developer", "end": END},
)
_builder.add_edge("fix_developer", "tester")
```

The `should_fix_or_end` function is pure Python: it parses the Tester's output and decides whether to rerun the Developer or finish. No LLM in the decision loop.

## The prompt caching problem and the switch to full Gemini

During the exploration phase, I very quickly hit **API rate limits** on Gemini. Every agent call sent the full system prompt, tool definitions, project context, thousands of tokens per request.

The solution: **prompt caching**. But Gemini and Claude handle it very differently.

### Gemini: implicit caching

Gemini automatically caches repeated prefixes. If the system prompt and initial instructions are identical between two calls, Google reuses the cached context. On the code side, there's nothing to do: caching is transparent.

```python
# Savings show up in usage metadata
cached = getattr(meta, "cached_content_token_count", 0)
total = getattr(meta, "prompt_token_count", 0)
logger.info("cache hit: %d/%d tokens (%d%%)", cached, total, cached * 100 // total)
```

### Claude: explicit caching

Claude requires explicit `cache_control: ephemeral` markers on the blocks you want cached: the system prompt, tool definitions, and the first user message.

```python
system = [{
    "type": "text",
    "text": self.instructions,
    "cache_control": {"type": "ephemeral"}
}]

claude_tools = [self._fn_to_claude_tool(fn) for fn in self.tools]
if claude_tools:
    claude_tools[-1]["cache_control"] = {"type": "ephemeral"}
```

### Why I switched to full Gemini

I started with a multi-LLM architecture: Gemini for the Architect and Tester, Claude for the Developer. The idea was appealing: use each LLM where it excels.

In practice, **Claude's API cost quickly made this approach unsustainable**. A full pipeline run with Claude as Developer cost significantly more than with Gemini, especially during fix iterations where the context grows with each turn. So I decided to switch to **full Gemini** as the default pipeline, while keeping the `ClaudeAgent` in the framework as a configurable option.

This pragmatic choice also let me fully benefit from Gemini's implicit caching across the entire pipeline, without managing two different caching strategies in production.

The contrast between both approaches still pushed me to design the class hierarchy to isolate these differences:

```
BaseAgent (ABC)
├── GeminiAgent    → implicit caching, google-genai SDK
│   ├── ArchitectAgent
│   ├── DesignerAgent
│   ├── DeveloperAgent
│   └── TesterAgent
└── ClaudeAgent    → explicit caching, anthropic SDK
    └── DeveloperAgent
```

Each agent inherits its backend's caching strategy without having to worry about it.

## Agent hierarchy: ABC and specialization

The core of the framework relies on a simple hierarchy:

- **`BaseAgent`** (ABC) — defines the contract: `run(context) → AgentResult`, tool management
- **`GeminiAgent`** — implements the agentic loop for Gemini (chat + tool calls)
- **`ClaudeAgent`** — implements the agentic loop for Claude (messages + tool_use blocks)

Specialized agents (Architect, Developer, Tester) inherit from `GeminiAgent` and only define their **instructions** and **tools**:

```python
class ArchitectAgent(GeminiAgent):
    def __init__(self):
        super().__init__(
            name="Architect",
            instructions="You are a software architect...",
            tools=[web_search, write_file],
            model_name="gemini-3.1-pro-preview",
        )
```

To add a new agent, just create a class, define its instructions, and add it as a node in the LangGraph pipeline. No need to touch the chat logic, tool calling, or caching.

## The Designer: a special case

The `DesignerAgent` is an interesting case. Unlike other agents that use the standard agentic loop (chat → tool call → response → tool call → ...), the Designer makes **direct API calls** to generate SVG.

Why? Because SVG generation is a well-defined two-step workflow:

1. **Planning** — "what assets does this project need?" → returns JSON
2. **Generation** — "generate these N SVG sprites" → returns parsable text

No need for an agentic loop with tools here. The Designer still inherits from `GeminiAgent` (for the API client and key validation), but it **overrides `run()`** with its own logic.

## The automatic fix loop

One of the most useful aspects of the pipeline is the **fix loop**. When the Tester detects failures, the Developer is relaunched in **FIX MODE**:

```python
def should_fix_or_end(state: PipelineState) -> Literal["fix", "end"]:
    if (
        _has_test_failures(state.get("test_results", ""))
        and state.get("fix_iterations", 0) < MAX_FIX_ITERATIONS
    ):
        return "fix"
    return "end"
```

The Developer then receives the test output in its context, with a clear instruction:

> *"You are in FIX MODE — read existing files and fix these. Do NOT rewrite all files from scratch."*

In practice, 3 iterations are enough in most cases to go from 60-70% passing tests to 100%.

## Shared tools

Agents interact with the file system through 4 simple tools:

| Tool | Role |
|---|---|
| `write_file(path, content)` | Write a file (creates parent directories) |
| `read_file(path)` | Read an existing file |
| `execute_code(command)` | Execute a shell command |
| `web_search(query)` | Web search via DuckDuckGo |

These tools are plain Python functions, passed to agents through their constructor. The framework handles exposing them to the LLM in the right format (Gemini function declarations or Claude tool definitions).

## The limits: a solid foundation, not a finished product

Let's be honest about what the framework can and can't do. **AgenticDev excels at generating a functional project base**: file structure, initial code, tests, documentation. For simple projects (CLI tools, libraries, small APIs), the output is often usable as-is.

But as complexity grows (intricate business logic, multiple integrations, performance constraints), **the generated code will be a starting point, not the final product**. There will be technical limitations (overly naive architectures, uncovered edge cases) and functional gaps (the LLM doesn't know your business context) that you'll need to fix manually or by vibe-coding with a tool like Claude Code or Cursor.

This is actually the workflow I recommend: let AgenticDev generate the skeleton, then iterate on it with a coding assistant to refine the details. The framework saves you the first hours of setup, not the last hours of polish.

## What I learned

### Specialization beats generality

An agent that "does everything" is less reliable than a team of specialized agents. The Architect can't code, the Developer can't test, and that's by design. Each agent has precise instructions and a limited scope.

### Deterministic orchestration is non-negotiable

Letting an LLM decide the execution flow means accepting that the pipeline behaves differently on every run. For a code generation tool, that's unacceptable. LangGraph let me keep the LLMs' creativity while enforcing a predictable execution order.

### Prompt caching is essential in multi-agent systems

Without caching, a 4-agent pipeline easily consumes 100k+ tokens per run, 80% of which is repeated context. Caching significantly reduces both costs and latency.

### Cost dictates architecture

Starting with multi-LLM was intellectually satisfying, but economic reality caught up. Keeping the multi-backend abstraction while using a single provider by default is the right trade-off: you only pay for what you use, without sacrificing flexibility.

### Agent instructions are code

Agent prompts aren't vague sentences: they're precise specifications with rules, examples, and edge cases. For instance, the Developer's prompt includes rules on Python vs TypeScript conventions, placeholder handling, and a mandatory completion audit before returning its response.

## Going further

The source code is available on [GitHub](https://github.com/HamdiMechelloukh/AgenticDev). The framework is designed to be extended: adding a new agent takes about ten lines of code.

You can also read this and other articles on [my portfolio](https://www.hamdimechelloukh.com).

Next steps I'm considering:
- Support for new LLM backends (Mistral, Llama)
- Quality metrics on generated code
- Interactive mode with human validation between each step
