# Release process

1. Create a task branch.
2. Run static compilation:

```powershell
python -m compileall src tests
```

3. Run tests:

```powershell
$env:PYTHONPATH = "src"
python -m unittest discover -s tests
```

4. Smoke-test the GUI on Windows 11.
5. Export a small test project using `safe`, `balanced`, `full` and `Codex Package` modes.
6. Verify that `.env`, private keys and `node_modules` are handled correctly.
7. Verify archive splitting with a temporary low ZIP limit.
8. Update README/docs if behaviour changed.
9. Merge into `main` only after checks pass.
10. Tag the version and keep the previous ZIP release available for rollback.
