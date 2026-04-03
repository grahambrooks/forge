# Architecture Forge

There are many approaches to documenting architectures.

* Markdown documentation with diagrams using something like mermaid.js.
* Structurizr and commercial tools that take a design first approach with an underlying model.
* ADRs document change but not state

Both of these approaches have there plusses and minuses.

Architecture often comes after implementation either from wanting to document an existing system or from a desire to
design a system after the fact. In both of these cases, the architecture is often not the source of truth but rather a
reflection of the implementation. This can lead to documentation that is out of date and not useful.

The modeling approach often focuses on a layered approach to documenting the software but does not cover the processes
and systems that are used to build and deploy the software.

Forge tackles these problems by

* Generating a model from existing project source code
* Represneting the model in a DSL that includes both software structure and the processes that are used to buid and
  deploy the projects.
* Providing a way to document change through comparisons with a baseline model.
* Supports composition - for example if there are multiple projects that share a tech stack (common in enterprise systems and microservices) the model can reference external specifications and include them in the design of the project.