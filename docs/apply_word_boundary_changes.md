# Applying the word-boundary keyword changes to your GitHub repo

Follow these steps to bring the previously implemented word-boundary keyword lexing updates into your own GitHub fork or repository.

## 1) Fetch and inspect the branch
1. Ensure you have a remote pointing at the repository that contains the changes (e.g., `origin`).
2. Fetch all branches: `git fetch origin`.
3. View the branch with the changes (replace the example branch name if different): `git log --oneline origin/claude/explore-codebase-hni18 | head`.

## 2) Create a local branch to integrate the work
1. Check out the branch with the changes: `git checkout -b claude/explore-codebase-hni18 origin/claude/explore-codebase-hni18`.
2. Run the test suite locally to validate the state: `cargo test` (note: microcode tests may fail without a `LanguageSchema` argument for `execute`).

## 3) Merge into your mainline
1. Switch to your main branch (e.g., `main`): `git checkout main`.
2. Merge the changes: `git merge --no-ff claude/explore-codebase-hni18`.
3. Resolve any conflicts, then rerun `cargo test`.

## 4) Push to GitHub
1. Push your updated main branch: `git push origin main`.
2. If you prefer a pull request workflow, push the feature branch instead: `git push origin claude/explore-codebase-hni18` and open a PR on GitHub comparing that branch to `main`.

## 5) (Optional) Clean up
- After merging, you can delete the local branch: `git branch -d claude/explore-codebase-hni18`.
- To delete the remote branch: `git push origin --delete claude/explore-codebase-hni18`.

## Notes
- The changes are already committed in the branch; no extra cherry-picks are required unless you only want a subset.
- If your repository uses a different default branch name (e.g., `master`), replace `main` with that name in the commands above.
- If tests fail due to the microcode API signature, align `execute` call sites with the expected `LanguageSchema` argument before merging.
