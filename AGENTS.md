# Setting

We are planning out the software for the time being. You should not write code until you are instructed to do so.
Your main job is to converse, ideate, and propose solutions.
This extends to `docs/ARCHITECTURE.md` too; should not be edited without permission.

Be critical, and don't be afraid to challenge ideas. Decisions we make today can have a long-lasting impact on the project, so we should be careful and deliberate.
There are many language servers out there, and core ideas and patterns are established in the industry. Language servers like rust-analyser, Roslyn, IDEA PSI, and Eclipse JDT are good references. We should learn from them, but at the same time open to new ideas and approaches.

## References

Before writing language-specific code, all relevant specification pages should be read and understood.

For copyright reasons, we don't commit MD-ified versions of the specifications, but executing `scripts/specs/update.py` provides access to:
- `docs/lang-specs/jls/se26/`, containing the JLS 26 in Markdown
- `docs/lang-specs/jvms/se26/`, containing the JVMS 26 in Markdown

Use these docs to drive the implementation, and provide citations to _why_ something is happening.

# Conversation style

Propose changes as text in the conversation; only touch the file when explicitly permitted.
Don't infodump; keep the conversation focused and structured. Ask rather impose.
I will be deliberate when it's time to make plans, write code, or make decisions.

Your default should be succinct ideation and chatting. If you want to propose a change, do so in the conversation first. If I agree, I will ask you to make the change in the file.

# Code style

Comments should be used very sparingly; this is an experimental project, things move, and I would like to add most of the comments strenthening my understanding.
I am not a Rust expert; be very critical of my code and suggest better ways to do things. I want to learn idiomatic Rust and best practices. 
I want to avoid cargo culting, so please explain why a change is better than what I have.

## Testing

Read the dedicated `docs/TESTING.md` to learn more about our testing policy.

## TODOs

The project is not yet using GH Issues as development is mostly single person and local.
For simplicity, there is a `TODO.md` file.
Make sure the file is maintained after a feature there is developed.