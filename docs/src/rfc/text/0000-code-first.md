# RFC: Patina Code First Process

This RFC defines a code first process for the Patina project. A code first process allows source code to be developed
in tandem with specification and design documents. This allows for a more iterative development process where design
decisions can be validated through implementation and testing before being finalized in documentation.

The Patina Code First process intends to be very lightweight reducing overhead for managing source code changes. While
this iteration of the RFC focuses on specifications maintained by the UEFI Forum, the process defined here is expected
to be updated/extended in the future to cover other specifications on an as needed basis.

## Change Log

- 2026-04-30: Initial RFC created.

## Motivation

As a firmware project, Patina must frequently implement source code for changes actively being defined in specifications,
particularly for new firmware capabilities and interfaces and hardware technologies. The code first process allows
Patina to implement and test code in parallel with specification drafts. This has the dual benefits of building confidence
in specification design decisions while also resulting in earlier implementation readiness.

In the case of the UEFI Forum, this process lets changes and development of new features happen in open source, without
violating the UEFI Forum bylaws which otherwise prevent publication of code for in-draft features/changes as they are
under UEFI NDA.

Finally, since Patina is a Rust project, code first implementation provides an opportunity to influence specification
design with insights from Rust language features and design patterns and even idiomatic Rust APIs. This is an important
step toward reducing the dependency on C language design patterns and APIs in UEFI specifications.

## Goals

Goal: Define a process that allows Patina source code to be developed alongside ECRs for UEFI Forum specifications.

## Unresolved Questions

- Outside the purview of Patina: But can the UEFI Forum maintain public specification source files (markdown/rst) in
  in a publicly accessible repository? That would make it much easier for developers writing draft changes for their
  code first change to copy, paste, and modify existing specification source files for their change.

## Prior Art

The most substantial prior art for this RFC is the [EDK II Code First Process](https://www.tianocore.org/tianocore-wiki.github.io/development/contribution-guides/edk_ii_code_first_process.html).
The most notable differences from that process is that Patina does not require source code annotation (e.g. code comments).

## Alternatives

1. Implement code changes after specification changes are finalized and published.
   - Rejected because: This results in a longer development process and delays feedback on design decisions until after
     code is implemented.
2. Implement code changes in a private repository.
   - Rejected because: This prevents the benefits of open source development and collaboration.
3. Implement code changes in a public repository without any process for formally describing specification
   modifications.
   - Rejected because: This would make it difficult to track exactly what the change is attempting to implement. All
     community members might not have access to the ECR. In addition to which code changes are related to which
     specification changes increasing likelihood of confusion an lack of coordination between code and specification
     development.

## The Process

The code first author:

1. Creates a new issue in the Patina repository using the "Code First" form.
   - Note: Ensure all specifications impacted by the change are selected in the form.
   - The issue must have the `type:code-first` label applied to it.
     - Note: This should happen automatically if the "Code First" form is used to create the issue.
2. Creates a local "code first" branch.
3. Writes a specification draft change in a markdown file included in a standalone commit on the "code first branch".
   - Note: A file must be present for each specification if more than one specification is impacted by the change.
   - Note: Base the file content on the template in the Code First GitHub issue submission form.
4. Authors the code first implementation in the same branch using one or more commits as appropriate.
5. Pushes the branch for the code first change to their fork (e.g. `username/patina`).
6. Creates a draft pull request into the default branch (`main`) of the `OpenDevicePartnership/patina` repository.
7. Applies the `type:code-first` label to the pull request.
8. Links the GitHub issue created in step 1 to the pull request.
   - Note: This can be done using a [keyword](https://docs.github.com/issues/tracking-your-work-with-issues/using-issues/linking-a-pull-request-to-an-issue#linking-a-pull-request-to-an-issue-using-a-keyword)
     in the pull request description (e.g. "Closes \#123") or by [manually linking](https://docs.github.com/issues/tracking-your-work-with-issues/using-issues/linking-a-pull-request-to-an-issue#manually-linking-a-pull-request-to-an-issue-using-the-pull-request-sidebar)
     the issue to the pull request using the GitHub UI.
9. Continues to develop the code change in the "code first branch" until it is ready for review.
10. Takes the PR out of draft after all dependent specification changes have been approved and are publicly published.
11. Goes through the normal PR review process until the PR is approved and merged.

### GitHub Code First Issue Template Example

```markdown
name: </> Code First
description: Code first tracking issue
title: "[Code First]: <title>"
labels: ["type:code-first"]

body:
  - type: markdown
    attributes:
      value: |
        Introductory text

  - type: textarea
    id: overview
    attributes:
      label: Code First Item Overview
      description: Provide a brief overview of the overall code first change.
    validations:
      required: true

  - type: dropdown
    id: specs_impacted
    attributes:
      label: What specification(s) are directly related?
      description: |
        *Select all that apply*
      multiple: true
      options:
        - ACPI
        - Platform Initialization (PI)
        - UEFI
        - UEFI PI Distribution Packaging
        - UEFI Shell
    validations:
      required: true

  - type: markdown
    attributes:
      value: |
        **Specification Draft Template**

        For the template below, the title and complete description of the specification changes must be provided in the
        specification text along with the name and version of the specification the change applies. The `Status` of the
        specification change always starts in the `Draft` state and is updated based on feedback from the industry
        standard forums. The contents of the specification text are required to use the
        [Creative Commons Attribution 4.0 International](https://spdx.org/licenses/CC-BY-4.0.html) license using a
        `SPDX-License-Identifier` statement.

        - "Required" sections must be completed.
        - Include a modified template for each specification impacted (if more than one).
        - Include a copy of the completed template in a markdown file in the code changes.
          - If more than one template is completed, place each in a separate markdown file.

        ---

        Template text for reference (using the GitHub flavor of markdown):

        ```markdown
        # Title: [Must be Filled In]

        ## Status: [Status]

        [Status] must be one of the following:
        - Draft
        - Submitted to industry standard forum
        - Accepted by industry standard forum
        - Accepted by industry standard forum with modifications
        - Rejected by industry standard forum

        ## Document: [Title and Version]

        Here are some examples of [Title and Version]:
        - UEFI Specification Version 2.8
        - ACPI Specification Version 6.3
        - UEFI Shell Specification Version 2.2
        - UEFI Platform Initialization Specification Version 1.7
        - UEFI Platform Initialization Distribution Packaging Specification Version 1.1

        ## License

        SPDX-License-Identifier: CC-BY-4.0

        ## Submitter: [Open Device Partnership](https://www.opendevicepartnership.org)

        ## Summary of the change

        Required Section

        ## Benefits of the change

        Required Section

        ## Impact of the change

        Required Section

        ## Detailed description of the change [normative updates]

        Required Section

        ## Special Instructions

        Optional Section
        ```

  - type: textarea
    id: anything_else
    attributes:
      label: Anything else?
      description: |
        Links? References? Anything that will give us more context about the code first change.

        Tip: You can attach images or log files by clicking this area to highlight it and then dragging files in.
    validations:
      required: false
```
