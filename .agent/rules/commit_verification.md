# Rule: Verification Before Commit

- **Strict Requirement**: You MUST NOT execute a `git commit` command unless the USER has explicitly verified the specific changes in that commit and provided a final approval to proceed with the commit.
- **No Assumptions**: Approval given for a previous set of changes does not apply to subsequent tweaks. Every individual edit or refinement MUST be verified by the USER before you commit it.
- **Verification Process**:
    1. Present the changes (via code diff, walkthrough update, or describing the visual result).
    2. Ask the USER for verification/approval.
    3. Only run `git commit` AFTER the USER says "commit it", "looks good", or similar explicit approval for that specific state of the code.
