
# Drasi Governance and Community Membership
This document details the governance policies for the Drasi project and outlines the responsibilities of contributor roles in Drasi. The Drasi project includes the platform and core components, SDKs, CLI, samples, QuickStart tutorials, and docs. Responsibilities for roles are scoped to each repo.

## Objectives
Drasi is a comprehensive open-source solution designed to detect and react intelligently to changes insoftware systems. Drasi acts as a single software component to integrate easily with existing data repositories via Sources and transmit evaluated change data to downstream consumers through Reactions.

## Code of Conduct

This project has adopted the [Contributor Covenant Code of Conduct](https://github.com/drasi-project/community/blob/main/CODE_OF_CONDUCT.md).

## Project Roles and Responsibilities

| **Project Role**   | **Responsibilities**                                  | **Requirements**                                             | **Defined by**                                               |
| ---------- | ----------------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------ |
| Member     | Active contributor in the community.  Reviewer of PRs | Sponsored by two approvers or maintainers. Multiple contributions to the project. | Drasi GitHub org member                           |
| Approver   | Approve accepting contributions                       | Highly experienced and active reviewer and contributor to a subproject. | [CODEOWNERS](https://help.github.com/en/articles/about-code-owners) in GitHub |
| Maintainer | Set direction and priorities for a subproject         | Demonstrated responsibility and excellent technical judgement for the subproject. | [CODEOWNERS](https://help.github.com/en/articles/about-code-owners), GitHub Team and repo ownership in GitHub |


<!--
## New contributors

New contributors should be welcomed to the community by existing members, helped
with PR workflow, and directed to relevant documentation and communication
channels.

## Established community members

Established community members are expected to demonstrate their adherence to the
principles in this document, familiarity with project organization, roles, policies, procedures, conventions, etc., and technical and/or writing ability.
Role-specific expectations, responsibilities, and requirements are enumerated
below.
--> 
## Member

Members are continuously active contributors in the community. They can have issues and PRs assigned to them. Members are expected to participate in community discussions and remain active contributors to the community.  

Defined by: Member of the Drasi GitHub organization

### Requirements

- Enabled [two-factor authentication](https://help.github.com/articles/about-two-factor-authentication) on their GitHub account
- Have made multiple contributions to the project or community. Contributions may include, but is not limited to:
  - Authoring or reviewing PRs on GitHub
  - Filing or commenting on issues on GitHub
  - Contributing to sub-projects, or community discussions (e.g. meetings,
    chat, etc.)
- [Joined the Discord server](https://aka.ms/drasidiscord)
- Have read the [contributor
  guide]( https://github.com/drasi-project/drasi-platform/blob/main/CONTRIBUTING.md)
- Actively contributing to 1 or more sub-projects
- Sponsored by two approvers or maintainers (sponsors). Note the following requirements for sponsors:
  - Sponsors must have close interactions with the prospective member - e.g. code/design/proposal review, coordinating on issues, etc
  - Sponsors must be approvers or maintainers in at least one repo in the Drasi org
- [Open an
  issue]( https://github.com/drasi-project/community/issues/new?assignees=&labels=community-membership&projects=&template=organization-membership-request.md&title=REQUEST%3A+New+membership+for+%3Cyour-GH-handle%3E)
  against the
  [Drasi/community](https://github.com/drasi-project/community) repo
  - Ensure your sponsors are `@mentioned` on the issue
  - Complete every item on the checklist [preview the current version of the
    template](https://github.com/drasi-project/community/issues/new?assignees=&labels=community-membership&projects=&template=organization-membership-request.md&title=REQUEST%3A+New+membership+for+%3Cyour-GH-handle%3E)
  - Make sure that the list of contributions included is representative of your work on the project
- Have your sponsoring reviewers reply confirmation of sponsorship: `+1`
- Once your sponsors have responded, your request will be reviewed by the Drasi Maintainers. Any member of the Maintainers team can review the requirements and add members to the GitHub org

### Responsibilities and privileges

- Responsive to issues and PRs assigned to them
- Active owner of code contributed by them (unless ownership is explicitly
  transferred)
  - Code is well tested
  - Tests consistently pass
  - Addresses bugs or issues discovered after code is accepted
  - Members are encouraged to review and approve via the GitHub workflow. This review work, while not sufficient to merge a PR, does accrue toward the Member becoming an Approver. Merge approval and final review are performed by an Approver
- Members can be assigned to issues and PRs, and people can ask members for reviews with a `/cc @username`
<!--
> Note: members who frequently contribute code are expected to proactively perform code reviews and work towards becoming an *approver* for the sub-projects in which they are active.  Acceptance of code contributions requires at least one approver in addition to the reviews by *members.*
-->
## Approver

Code approvers can both review and approve code contributions, as well as help maintainers triage issues and do project management.

While code review is focused on code quality and correctness, approval is
focused on holistic acceptance of a contribution including backwards/forwards
compatibility, adhering to API and conventions, performance and correctness issues, interactions with other sub-projects, etc.

Approver status can be scoped to a part of the codebase. For example, critical core components may have higher bar for becoming an approver.

### Requirements

The following apply to the part of the codebase for which one would be an Approver:
- Reviewer of the codebase for at least 1 month
- Reviewer for at least 5 substantial PRs to the codebase, with the definition of substantial area to the maintainer's discretion (e.g. refactors/adds new functionality rather than one-line pulls)
- Nominated by a maintainer from the repository in which the nomination is applied to:
  - With an approving vote of at least 2 maintainers from the repository maintainers. In the case of a repository with a solo maintainer, a single vote suffices.
  - With no objections from other repository maintainers for a period of one week.
  - Maintainers team acts as the final resolution to any escalation.

### Responsibilities and privileges

The following apply to the part of the codebase for which one would be an approver in the `CODEOWNERS` files:

- Approver status may be a precondition to accepting large code contributions
- Demonstrate sound technical judgement (may be asked to step down by a maintainer if they lose confidence of the maintainers)
- Responsible for project quality control via code reviews
  - Focus on holistic acceptance of contribution such as dependencies with other
    features, backwards / forwards compatibility, API and conventions, etc
- Expected to be responsive to review requests (inactivity for more than 1 month may result in suspension until active again)
- Mentor contributors and reviewers
- May approve code contributions for acceptance
- Inactivity for more than 3 months leads to a removal vote by other maintainers/approvers and not an automatic removal

## Maintainer

Maintainers are the technical authority for a subproject in the Drasi org. They *MUST* have demonstrated both good judgement and responsibility towards the health of that subproject. Maintainers *MUST* set technical direction and make or approve design decisions for their subproject - either directly or through delegation of these responsibilities.

Defined by: GitHub organization ownership, permissions and membership in the Maintainers team.

### Requirements

The following apply to the project for which one would be an owner in the `CODEOWNERS` files:


- Deep understanding of the technical goals and direction of the subproject
- Deep understanding of the technical domain (specifically the language) of the
  project
- Sustained contributions to design and direction by doing all of:
  - Authoring and reviewing proposals
  - Initiating, contributing and resolving discussions (e.g. emails, GitHub issues, meetings)
  - Identifying subtle or complex issues in designs and implementation PRs
- Directly contributed to the project through implementation and / or review
<!-- Aligning with the overall project goals, specifications and design principles defined by the Maintainers team. Bringing general questions and requests to the discussions as part of specifications  -->
<!-- Must have been active for 3 months or more for the given project
- Inactivity for more than 3 months leads to a removal vote by other maintainers and not an automatic removal-->

<!---
### Acceptance

New maintainers can be added to the project by a super-majority (two-thirds / 66.66%) vote. Only the maintainers of the repository in which the nomination is applied to have a binding vote, while maintainers from other repositories are on an informed basis via a separate email thread. A potential maintainer may be nominated by an existing maintainer from the repository in which the nomination is applied to. A vote is conducted in private between the current maintainers over the course of a one week voting period. At the end of the week, votes are counted, and a pull request is made on the repo adding the new maintainer to the CODEOWNERS file.

Maintainers for new repositories can be proposed by any maintainer and are elected by a ⅔ majority maintainers vote. Maintainers can be removed by a ⅔ majority maintainers vote.
Single maintainers of a repository can nominate a new maintainer and *MUST* inform other maintainers of their intention. The maintainer can be approved if no objections have been raised in a period of one week.

A maintainer may step down by submitting an issue stating their intent.
-->
### Responsibilities and privileges

The following apply to the repos for which one would be an owner:

- Make and approve technical design decisions for the project
- Set technical direction and priorities for the project
- Define milestones and releases
  - Decides on when PRs are merged to control the release scope
- Mentor and guide approvers and contributors to the project
- Escalate *approver* and *maintainer* workflow concerns (i.e. responsiveness, availability, and general contributor community health) to the TC
- Ensure continued health of subproject:
  - Adequate test coverage to confidently release
  - Tests are passing reliably (i.e. not flaky) and are fixed when they fail
- Ensure a healthy process for discussion and decision making is in place
- Work with other maintainers to maintain the project's overall health and success holistically

Maintainers *MUST* remain active. If they are unresponsive for >3 months, they will be automatically removed unless a super-majority of the other repository maintainers agrees to extend the period to be greater than 3 months.
