"""pipeline.py stages with the subprocess runner replaced (no network, no git, no cargo)."""
import contextlib
import datetime
import io
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

DATAGEN = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(DATAGEN))
os.environ["NAVZEROTH_DATAGEN_OFFLINE"] = "1"
import pipeline  # noqa: E402

HEADER = ("Branch!STRING:0|Active!DEC:1|Build Key!HEX:16|CDN Key!HEX:16|Install Key!HEX:16|"
          "IM Size!DEC:4|CDN Path!STRING:0|CDN Hosts!STRING:0|CDN Servers!STRING:0|Tags!STRING:0|"
          "Armadillo!STRING:0|Last Activated!STRING:0|Version!STRING:0|KeyRing!HEX:16|Product!STRING:0")


def row(active, version, product, locale="enUS"):
    tags = f"Windows x86_64 EU? acct-FRA? geoip-FR? {locale} speech?:Windows x86_64 EU? {locale} text?"
    return f"eu|{active}|aa|bb||||||{tags}|||{version}||{product}"


def build_info(*rows):
    return "\n".join([HEADER, *rows]) + "\n"


TARGET = {
    "format": "navzeroth-world-data-target", "schema_version": 1, "decision": "ADR 0007 §6",
    "edition": "wow_forever", "build": "1.60.1.70450", "interface": "16001", "locale": "enUS",
    "detected_from": "owner-recorded development build",
}


class FakeRunner:
    """Records commands; answers with the first matching (predicate, CompletedProcess)."""

    def __init__(self, answers=()):
        self.calls = []
        self.answers = list(answers)

    def __call__(self, cmd, cwd, env=None, log=None):
        self.calls.append((cmd, env))
        for match, (code, out) in self.answers:
            if match(cmd):
                return subprocess.CompletedProcess(cmd, code, out, "")
        return subprocess.CompletedProcess(cmd, 0, "", "")

    def commands(self):
        return [c for c, _ in self.calls]

    def ran(self, *words):
        return [c for c in self.commands() if all(w in c for w in words)]


def has(*words):
    return lambda cmd: all(w in cmd for w in words)


class Repo:
    """A temporary repository root with the committed target file."""

    def __init__(self, tmp):
        self.root = pathlib.Path(tmp)
        (self.root / "release" / "v1").mkdir(parents=True)
        self.target = self.root / "release" / "v1" / "world-data-target.json"
        self.target.write_text(json.dumps(TARGET, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def context(root, argv=(), runner=None, probes=(), local=None):
    args = pipeline.parse_args(list(argv))
    ctx = pipeline.Context(args, runner=runner or FakeRunner(), root=root, probes=list(probes),
                           local=local or pathlib.Path(root) / "local.json",
                           now=datetime.datetime(2026, 10, 19, tzinfo=datetime.timezone.utc))
    ctx.logs = pathlib.Path(root) / "logs"
    return ctx


def quiet(fn, *a):
    with contextlib.redirect_stdout(io.StringIO()):
        return fn(*a)


class BuildInfo(unittest.TestCase):
    def test_parses_columns_by_bare_name(self):
        rows = pipeline.parse_build_info(build_info(row("1", "1.60.1.70450", "wow_classic_beta")))
        self.assertEqual(rows[0]["Product"], "wow_classic_beta")
        self.assertEqual(rows[0]["Version"], "1.60.1.70450")
        self.assertEqual(rows[0]["Active"], "1")

    def test_skips_malformed_and_blank_lines(self):
        text = build_info(row("1", "1.60.1.70450", "wowf")) + "short|row\n\n"
        self.assertEqual(len(pipeline.parse_build_info(text)), 1)
        self.assertEqual(pipeline.parse_build_info(""), [])

    def test_chooses_active_row(self):
        rows = pipeline.parse_build_info(build_info(
            row("0", "1.60.0.70000", "wow_classic_beta"),
            row("1", "1.60.1.70450", "wow_classic_beta")))
        self.assertEqual(pipeline.choose_row(rows)["Version"], "1.60.1.70450")

    def test_prefers_1_60_version_then_product_order(self):
        rows = pipeline.parse_build_info(build_info(
            row("1", "11.2.5.63000", "wow"),
            row("1", "1.15.7.61000", "wow_classic_beta"),
            row("1", "1.60.1.70450", "wowf")))
        self.assertEqual(pipeline.choose_row(rows)["Product"], "wowf")
        rows = pipeline.parse_build_info(build_info(
            row("1", "1.61.0.71000", "wowf"), row("1", "1.61.0.71001", "wow_classic_beta")))
        self.assertEqual(pipeline.choose_row(rows)["Product"], "wow_classic_beta")

    def test_no_forever_row(self):
        rows = pipeline.parse_build_info(build_info(row("1", "11.2.5.63000", "wow"),
                                                    row("0", "1.60.1.70450", "wowf")))
        self.assertIsNone(pipeline.choose_row(rows))

    def test_interface(self):
        self.assertEqual(pipeline.interface_of("1.60.1.70450"), "16001")
        self.assertEqual(pipeline.interface_of("1.15.7.61000"), "11507")
        self.assertEqual(pipeline.interface_of("11.2.5.63000"), "110205")
        with self.assertRaises(ValueError):
            pipeline.interface_of("1.60")

    def test_locale_from_tags(self):
        self.assertEqual(pipeline.locale_of({"Tags": "x deDE text?"}, "enUS"), "deDE")
        self.assertEqual(pipeline.locale_of({"Tags": ""}, "enUS"), "enUS")


class Detect(unittest.TestCase):
    def install(self, base, name, *rows):
        folder = pathlib.Path(base) / name
        folder.mkdir(parents=True)
        (folder / ".build.info").write_text(build_info(*rows), encoding="utf-8")
        return folder

    def test_probe_order_first_found_wins(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Repo(tmp)
            missing = pathlib.Path(tmp) / "c" / "World of Warcraft"
            second = self.install(pathlib.Path(tmp) / "d", "World of Warcraft", row("1", "1.60.2.71000", "wowf"))
            third = self.install(pathlib.Path(tmp) / "e", "World of Warcraft", row("1", "1.60.3.72000", "wowf"))
            ctx = context(repo.root, probes=[missing, second, third])
            path, how, probed = pipeline.find_build_info(ctx)
            self.assertEqual(path, second / ".build.info")
            self.assertEqual(how, "probe")
            self.assertEqual(probed, [str(missing), str(second)])
            status, note = quiet(pipeline.stage_detect, ctx)
            self.assertEqual(status, "ok")
            doc = json.loads(repo.target.read_text(encoding="utf-8"))
            self.assertEqual((doc["build"], doc["interface"]), ("1.60.2.71000", "16002"))
            self.assertEqual(list(doc), list(TARGET))
            self.assertNotIn(tmp, doc["detected_from"])
            self.assertIn("'World of Warcraft'", doc["detected_from"])

    def test_product_subfolder_uses_parent(self):
        with tempfile.TemporaryDirectory() as tmp:
            install = self.install(tmp, "World of Warcraft", row("1", "1.60.1.70450", "wow_classic_beta"))
            (install / "_classic_beta_").mkdir()
            self.assertEqual(pipeline.build_info_in(install / "_classic_beta_"), install / ".build.info")

    def test_client_flag_beats_probes_and_must_exist(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Repo(tmp)
            probe = self.install(pathlib.Path(tmp) / "p", "World of Warcraft", row("1", "1.60.2.71000", "wowf"))
            chosen = self.install(pathlib.Path(tmp) / "q", "WoW", row("1", "1.60.3.72000", "wowf"))
            ctx = context(repo.root, ["--client", str(chosen)], probes=[probe])
            self.assertEqual(pipeline.find_build_info(ctx)[0], chosen / ".build.info")
            ctx = context(repo.root, ["--client", str(pathlib.Path(tmp) / "nowhere")], probes=[probe])
            with self.assertRaises(pipeline.StageError):
                pipeline.find_build_info(ctx)

    def test_local_json_fallback(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Repo(tmp)
            install = self.install(pathlib.Path(tmp) / "x", "Games WoW", row("1", "1.60.1.70451", "wowf"))
            local = repo.root / "local.json"
            local.write_text(json.dumps({"client": str(install)}), encoding="utf-8")
            ctx = context(repo.root, probes=[pathlib.Path(tmp) / "none"], local=local)
            path, how, _ = pipeline.find_build_info(ctx)
            self.assertEqual((path, how), (install / ".build.info", "local.json"))
            local.write_text("{}", encoding="utf-8")
            with self.assertRaises(pipeline.StageError):
                pipeline.find_build_info(ctx)

    def test_nothing_found_keeps_target(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Repo(tmp)
            before = repo.target.read_bytes()
            ctx = context(repo.root, probes=[pathlib.Path(tmp) / "none"])
            status, note = quiet(pipeline.stage_detect, ctx)
            self.assertEqual(status, "skipped")
            self.assertIn("committed target kept", note)
            self.assertEqual(repo.target.read_bytes(), before)

    def test_unchanged_build_leaves_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Repo(tmp)
            install = self.install(tmp, "World of Warcraft", row("1", "1.60.1.70450", "wowf"))
            ctx = context(repo.root, probes=[install])
            quiet(pipeline.stage_detect, ctx)
            first = repo.target.read_bytes()
            status, note = quiet(pipeline.stage_detect, ctx)
            self.assertEqual((status, repo.target.read_bytes()), ("ok", first))
            self.assertIn("unchanged", note)

    def test_no_forever_row_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Repo(tmp)
            install = self.install(tmp, "World of Warcraft", row("1", "11.2.5.63000", "wow"))
            ctx = context(repo.root, probes=[install])
            with self.assertRaises(pipeline.StageError):
                quiet(pipeline.stage_detect, ctx)


class Arguments(unittest.TestCase):
    def test_default_order_and_only(self):
        self.assertEqual(pipeline.parse_args([]).stages, pipeline.STAGES)
        self.assertEqual(pipeline.parse_args(["--only", "gate,detect"]).stages, ["detect", "gate"])
        self.assertEqual(pipeline.parse_args(["--no-publish"]).stages, pipeline.STAGES[:-1])
        self.assertEqual(pipeline.parse_args(["--skip", "detect", "--only", "detect,verify"]).stages, ["verify"])

    def test_unknown_stage_rejected(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            pipeline.parse_args(["--only", "detect,deploy"])
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            pipeline.parse_args(["--only", "publish", "--no-publish"])

    def test_offline_and_refresh_exclusive(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            pipeline.parse_args(["--offline", "--refresh"])


class Stages(unittest.TestCase):
    def test_generate_commands(self):
        with tempfile.TemporaryDirectory() as tmp:
            runner = FakeRunner()
            ctx = context(Repo(tmp).root, ["--offline", "--pages-dir", "/saved", "--no-network-sources"], runner)
            quiet(pipeline.stage_generate, ctx)
            cmds = runner.commands()
            self.assertTrue(cmds[0][-1] == "/saved" and cmds[0][2].endswith("import_wowhead_pages.py"))
            self.assertTrue(cmds[1][2].endswith("run_all.py"))
            self.assertIn("--offline", cmds[1])
            self.assertEqual(cmds[1][cmds[1].index("--only") + 1], "gen_fishing,gen_world_cmangos,gen_world_db2")

    def test_cli_built_once_and_reused(self):
        with tempfile.TemporaryDirectory() as tmp:
            runner = FakeRunner([(has("verify-world-data"), (0, '{"files": 3, "records": 9}'))])
            ctx = context(Repo(tmp).root, [], runner)
            self.assertEqual(quiet(pipeline.stage_verify, ctx), ("ok", "files 3, records 9"))
            quiet(pipeline.stage_verify, ctx)
            self.assertEqual(len(runner.ran("cargo", "build")), 1)
            self.assertEqual(len(runner.ran("verify-world-data", "--input", "data-generators/output")), 2)

    def test_given_cli_is_used(self):
        with tempfile.TemporaryDirectory() as tmp:
            runner = FakeRunner()
            ctx = context(Repo(tmp).root, ["--cli", "/bin/azeroth-cli"], runner)
            quiet(pipeline.stage_verify, ctx)
            self.assertEqual(runner.commands(), [["/bin/azeroth-cli", "verify-world-data", "--input",
                                                  "data-generators/output"]])

    def test_verify_failure_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            runner = FakeRunner([(has("verify-world-data"), (1, "{}"))])
            ctx = context(Repo(tmp).root, ["--cli", "cli"], runner)
            with self.assertRaises(pipeline.StageError):
                quiet(pipeline.stage_verify, ctx)

    def test_missing_scripts_named(self):
        with tempfile.TemporaryDirectory() as tmp:
            ctx = context(Repo(tmp).root, ["--cli", "cli"])
            with self.assertRaisesRegex(pipeline.StageError, r"register_facts\.py is missing.*V2\.8c"):
                pipeline.stage_register(ctx)
            with self.assertRaisesRegex(pipeline.StageError, r"world_to_content\.py is missing.*V2\.8d"):
                pipeline.stage_content(ctx)

    def test_register_and_content_commands(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Repo(tmp)
            (repo.root / "data-generators").mkdir()
            for name in ("register_facts.py", "world_to_content.py"):
                (repo.root / "data-generators" / name).write_text("", encoding="utf-8")
            stale = repo.root / "dist" / "world-pack" / "1.60.1.70450"
            stale.mkdir(parents=True)
            runner = FakeRunner()
            ctx = context(repo.root, ["--cli", "cli"], runner)
            quiet(pipeline.stage_register, ctx)
            quiet(pipeline.stage_content, ctx)
            reg = runner.ran("--facts")[0]
            for flag in ("--registry", "--plan", "--objects", "--updates", "--notice"):
                self.assertIn(flag, reg)
            self.assertEqual(reg[reg.index("--target") + 1], "release/v1/world-data-target.json")
            self.assertTrue(runner.ran("cli", "source-sync"))
            self.assertTrue(runner.ran("cli", "source-build", "--output", "dist/source-knowledge-world"))
            content = runner.ran("--authored")[0]
            self.assertEqual(content[content.index("--out") + 1], "content/world/1.60.1.70450")
            self.assertTrue(runner.ran("cli", "build-content-v2", "--output", "dist/world-pack/1.60.1.70450"))
            self.assertTrue(runner.ran("cli", "verify-pack", "--input", "dist/world-pack/1.60.1.70450"))
            self.assertFalse(stale.exists())

    def test_gate_not_ready_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Repo(tmp)
            report = {"publication_allowed": False, "blockers": ["a", "b"]}
            runner = FakeRunner([(has("release-readiness"), (1, json.dumps(report)))])
            ctx = context(repo.root, ["--cli", "cli"], runner)
            status, note = quiet(pipeline.stage_gate, ctx)
            self.assertEqual(status, "ok")
            self.assertIn("NOT ready; 2 blockers", note)
            saved = repo.root / "content" / "world" / "1.60.1.70450" / "release-readiness.json"
            self.assertEqual(json.loads(saved.read_text(encoding="utf-8")), report)
            self.assertTrue(runner.ran("release-readiness", "--root", ".", "--version", pipeline.workspace_version()))

    def test_gate_without_report_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            runner = FakeRunner([(has("release-readiness"), (2, "usage"))])
            with self.assertRaises(pipeline.StageError):
                quiet(pipeline.stage_gate, context(Repo(tmp).root, ["--cli", "cli"], runner))


class Publish(unittest.TestCase):
    def repo(self, tmp):
        repo = Repo(tmp)
        (repo.root / "data-generators" / "output").mkdir(parents=True)
        return repo

    def test_skipped_when_nothing_changed(self):
        with tempfile.TemporaryDirectory() as tmp:
            runner = FakeRunner([(has("rev-parse", "HEAD"), (0, "base\n"))])
            ctx = context(self.repo(tmp).root, [], runner)
            status, note = quiet(pipeline.stage_publish, ctx)
            self.assertEqual((status, note), ("skipped", "no changes versus HEAD"))
            self.assertFalse(runner.ran("push"))
            self.assertFalse(runner.ran("gh"))
            add = runner.ran("git", "add")[0]
            self.assertIn("data-generators/output", add)
            self.assertIn("release/v1/world-data-target.json", add)
            env = runner.calls[[c for c, _ in runner.calls].index(add)][1]
            self.assertIn("GIT_INDEX_FILE", env)

    def test_readiness_report_alone_does_not_publish(self):
        with tempfile.TemporaryDirectory() as tmp:
            runner = FakeRunner([(has("diff", "--cached"), (0, "content/world/b/release-readiness.json\0"))])
            status, _ = quiet(pipeline.stage_publish, context(self.repo(tmp).root, [], runner))
            self.assertEqual(status, "skipped")
            self.assertFalse(runner.ran("push"))

    def test_new_branch_draft_pr(self):
        with tempfile.TemporaryDirectory() as tmp:
            runner = FakeRunner([
                (has("rev-parse", "HEAD"), (0, "base\n")),
                (has("diff", "--cached"), (0, "data-generators/output/run-manifest.json\0")),
                (has("check-attr"), (0, "data-generators/output/run-manifest.json\0filter\0lfs\0")),
                (has("cat-file", "-s"), (0, "130\n")),
                (has("cat-file", "blob"), (0, "version https://git-lfs.github.com/spec/v1\noid sha256:x\n")),
                (has("write-tree"), (0, "tree\n")),
                (has("fetch"), (128, "")),
                (has("commit-tree"), (0, "c0ffee123456\n")),
            ])
            status, note = quiet(pipeline.stage_publish, context(self.repo(tmp).root, [], runner))
            self.assertEqual(status, "ok", note)
            branch = "data/world-1.60.1.70450-20261019"
            commit = runner.ran("commit-tree")[0]
            self.assertEqual(commit[commit.index("-p") + 1], "base")
            self.assertEqual(commit.count("-p"), 1)
            self.assertTrue(runner.ran("push", "origin", f"c0ffee123456:refs/heads/{branch}"))
            self.assertFalse([c for c in runner.ran("push") if "--force" in c or "main" in " ".join(c)])
            self.assertTrue(runner.ran("gh", "pr", "create", "--draft", "--head", branch))

    def test_existing_branch_with_same_output_skips_and_lfs_checked(self):
        with tempfile.TemporaryDirectory() as tmp:
            runner = FakeRunner([
                (has("diff", "--cached"), (0, "data-generators/output/a.json\0")),
                (has("check-attr"), (0, "data-generators/output/a.json\0filter\0lfs\0")),
                (has("cat-file", "-s"), (0, "130\n")),
                (has("cat-file", "blob"), (0, "version https://git-lfs.github.com/spec/v1\n")),
                (has("rev-parse", "FETCH_HEAD"), (0, "tip\n")),
            ])
            ctx = context(self.repo(tmp).root, ["--branch", "data/world-weekly"], runner)
            status, note = quiet(pipeline.stage_publish, ctx)
            self.assertEqual(status, "skipped")
            self.assertIn("already holds", note)
            self.assertFalse(runner.ran("push"))

    def test_existing_branch_gets_merge_commit_and_pr_update(self):
        with tempfile.TemporaryDirectory() as tmp:
            runner = FakeRunner([
                (has("rev-parse", "HEAD"), (0, "base\n")),
                (has("diff", "--cached"), (0, "content/world/b/catalog.json\0")),
                (has("check-attr"), (0, "content/world/b/catalog.json\0filter\0unspecified\0")),
                (has("write-tree"), (0, "tree\n")),
                (has("rev-parse", "FETCH_HEAD"), (0, "tip\n")),
                (has("diff", "tip", "tree"), (0, "content/world/b/catalog.json\0")),
                (has("commit-tree"), (0, "abc\n")),
                (has("gh", "pr", "list"), (0, "42\n")),
            ])
            ctx = context(self.repo(tmp).root, ["--branch", "data/world-weekly"], runner)
            status, note = quiet(pipeline.stage_publish, ctx)
            self.assertEqual(status, "ok", note)
            commit = runner.ran("commit-tree")[0]
            self.assertEqual([commit[i + 1] for i, w in enumerate(commit) if w == "-p"], ["tip", "base"])
            self.assertTrue(runner.ran("gh", "pr", "edit", "42"))
            self.assertFalse(runner.ran("gh", "pr", "create"))

    def test_full_content_in_lfs_path_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            runner = FakeRunner([
                (has("diff", "--cached"), (0, "data-generators/output/a.json\0")),
                (has("check-attr"), (0, "data-generators/output/a.json\0filter\0lfs\0")),
                (has("cat-file", "-s"), (0, "900000\n")),
            ])
            with self.assertRaisesRegex(pipeline.StageError, "LFS"):
                quiet(pipeline.stage_publish, context(self.repo(tmp).root, [], runner))

    def test_refuses_non_data_branch(self):
        with tempfile.TemporaryDirectory() as tmp:
            for branch in ("main", "feature/x"):
                ctx = context(self.repo(tmp).root if branch == "main" else pathlib.Path(tmp), ["--branch", branch])
                with self.assertRaises(pipeline.StageError):
                    pipeline.stage_publish(ctx)


class Driver(unittest.TestCase):
    def test_stops_at_failure_and_logs(self):
        with tempfile.TemporaryDirectory() as tmp:
            runner = FakeRunner([(has("verify-world-data"), (1, ""))])
            ctx = context(Repo(tmp).root, ["--cli", "cli", "--only", "generate,verify,register"], runner)
            results = quiet(pipeline.run_stages, ctx)
            self.assertEqual([(r[0], r[1]) for r in results],
                             [("generate", "ok"), ("verify", "failed"), ("register", "not run")])
            self.assertIn("verify-world-data", (ctx.logs / "pipeline-verify.log").read_text(encoding="utf-8"))
            text = pipeline.summary(results, ctx.logs)
            self.assertIn("verify     failed", text)


if __name__ == "__main__":
    unittest.main()


class WorkspaceVersionTest(unittest.TestCase):
    def test_default_gate_version_follows_cargo_toml(self):
        import re
        want = re.search(r'^version\s*=\s*"([^"]+)"', (pipeline.ROOT / "Cargo.toml").read_text(), re.M).group(1)
        self.assertEqual(pipeline.workspace_version(), want)
        self.assertEqual(pipeline.parse_args([]).version, want)
