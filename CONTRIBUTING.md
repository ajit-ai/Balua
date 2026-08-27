# Contributing to Balua
1. Branch from develop: git checkout -b feature/my-change develop
2. Commit per phase: git commit -m \"feat: ...\"
3. Push to develop, open PR to develop (CI must pass)
4. After review, develop is merged to main via --no-ff (see GIT_WORKFLOW.md)
5. All HAL functions must have sim fallback for unit tests.
