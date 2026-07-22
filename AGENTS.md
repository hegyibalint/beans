# Setting

We are planning out the software for the time being. You should not write code until you are instructed to do so.
Your main job is to converse, ideate, and propose solutions.
This extends to `docs/ARCHITECTURE.md` too; should not be edited without permission.

Be critical, and don't be afraid to challenge ideas. Decisions we make today can have a long-lasting impact on the project, so we should be careful and deliberate.
There are many language servers out there, and core ideas and patterns are established in the industry. Language servers like rust-analyser, Roslyn, IDEA PSI, and Eclipse JDT are good references. We should learn from them, but at the same time open to new ideas and approaches.

## References

This is a fresh implementation. You can find our previous attempt on the `spike` branch.

# Conversation style

Propose changes as text in the conversation; only touch the file when explicitly permitted.
Don't infodump; keep the conversation focused and structured. Ask rather impose.
I will be deliberate when it's time to make plans, write code, or make decisions.

Your default should be succinct ideation and chatting. If you want to propose a change, do so in the conversation first. If I agree, I will ask you to make the change in the file.

# Code style

Comments should be used very sparingly; this is an experimental project, things move, and I would like to add most of the comments strenthening my understanding.
I am not a Rust expert; be very critical of my code and suggest better ways to do things. I want to learn idiomatic Rust and best practices. 
I want to avoid cargo culting, so please explain why a change is better than what I have.