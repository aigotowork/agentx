#!/usr/bin/env python3
"""Exercise the production cohort generator with real Cargo in a tiny disposable crate.

This verifies generator/Cargo invalidation, not product protocol or latency.
No root features, dependencies, toolchains or installed programs are changed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

STAMP = re.compile(r'native-build-v1:[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    source = args.source.resolve()
    generator = (source/'build.rs').read_bytes()
    assert not args.output.exists(), 'preserve prior receipt'
    environment = dict(os.environ, CARGO_NET_OFFLINE='true')
    environment.pop('AGENTX_NATIVE_BUILD_ID', None)
    records = []
    with tempfile.TemporaryDirectory(prefix='agentx-cohort-cargo-') as temporary:
        root = Path(temporary)
        target = root/'target'
        for directory in ['src', 'tests/support', '.cargo']:
            (root/directory).mkdir(parents=True)
        (root/'build.rs').write_bytes(generator)
        (root/'src/main.rs').write_text('fn main() { println!("{}", env!("AGENTX_NATIVE_BUILD_ID")); }\n')
        (root/'tests/support/embedded.txt').write_text('initial embedded asset\n')
        (root/'.cargo/config.toml').write_text('[build]\n')
        toolchain = subprocess.check_output(['rustup', 'show', 'active-toolchain'], text=True, env=environment).split()[0]
        compiler = subprocess.check_output(['rustup', 'which', 'rustc'], text=True, env=environment).strip()
        version = subprocess.check_output([compiler, '-vV'], text=True, env=environment)
        host = next(line.split(': ', 1)[1] for line in version.splitlines() if line.startswith('host: '))
        (root/'rust-toolchain.toml').write_text('[toolchain]\nchannel = '+json.dumps(toolchain)+'\n')
        (root/'Cargo.toml').write_text('[package]\nname = "agentx-native-cohort-probe"\nversion = "0.0.0"\nedition = "2024"\n[build-dependencies]\nuuid = { version = "=1.28.0", features = ["v4"] }\n[features]\ncohort_probe = []\n[profile.release]\nopt-level = 0\n')

        def run(command, env=None, expect_success=True):
            started = time.monotonic()
            result = subprocess.run(command, cwd=root, env=env or environment, capture_output=True, text=True, timeout=90)
            row = {'command': command, 'code': result.returncode, 'elapsed_seconds': time.monotonic()-started,
                   'stdout': result.stdout, 'stderr': result.stderr}
            assert (result.returncode == 0) == expect_success, row
            return row

        records.append({'case':'resolve_existing_uuid_subgraph', **run(['cargo', 'generate-lockfile', '--offline'])})
        previous = None

        def build(case, flags=(), env=None, same=False):
            nonlocal previous
            result = run(['cargo', 'build', '--locked', '--offline', '--target-dir', str(target), *flags], env)
            profile = 'release' if '--release' in flags else 'debug'
            binary = target/host/profile/'agentx-native-cohort-probe' if '--target' in flags else target/profile/'agentx-native-cohort-probe'
            stamp = run([str(binary)], env)['stdout'].strip()
            assert STAMP.fullmatch(stamp), stamp
            if previous is not None:
                assert (stamp == previous) == same, (case, previous, stamp, result)
            records.append({'case':case, 'stamp':stamp, 'same_as_previous':same if previous else None, **result})
            previous = stamp

        build('initial_actual_generator')
        build('unchanged_cargo_cache', same=True)
        spoof = dict(environment, AGENTX_NATIVE_BUILD_ID='native-build-v1:00000000-0000-4000-8000-000000000000')
        build('incoming_build_and_runtime_override_ignored', env=spoof, same=True)
        changes = [
            ('dirty_native_source', 'src/main.rs', '// dirty native source\n'),
            ('embedded_asset', 'tests/support/embedded.txt', 'changed embedded asset\n'),
            ('manifest', 'Cargo.toml', '\n# dirty manifest\n'),
            ('lockfile', 'Cargo.lock', '\n# dirty lock\n'),
            ('generator', 'build.rs', '\n// dirty generator\n'),
            ('same_effective_toolchain_file', 'rust-toolchain.toml', '\n# same installed toolchain\n'),
            ('cargo_configuration', '.cargo/config.toml', '\n# dirty cargo configuration\n'),
        ]
        for case, name, suffix in changes:
            with (root/name).open('a') as stream:
                stream.write(suffix)
            build(case)
        # A feature exists only in this minimal harness. This does not declare
        # any product feature or claim another toolchain/architecture was run.
        build('cargo_feature_unit', flags=['--features', 'cohort_probe'])
        build('explicit_same_host_target_unit', flags=['--target', host])
        build('cargo_release_profile_unit', flags=['--release'])
        flags_environment = dict(environment, RUSTFLAGS='--cfg agentx_cohort_probe')
        build('user_rustflags_cfg', env=flags_environment)
        build('explicit_actual_compiler_path', env=dict(flags_environment, RUSTC=compiler))
        wrapper = root/'rustc-wrapper.py'
        wrapper.write_text('#!/usr/bin/python3\nimport os, sys\nos.execv(sys.argv[1], sys.argv[1:])\n')
        wrapper.chmod(0o700)
        build('compiler_wrapper_input', env=dict(flags_environment, RUSTC=compiler, RUSTC_WRAPPER=str(wrapper)))
        build('declared_same_installed_toolchain_input', env=dict(flags_environment, RUSTC=compiler, RUSTUP_TOOLCHAIN=toolchain))
        records.append({'case':'clean_owned_package', **run(['cargo', 'clean', '--target-dir', str(target), '--package', 'agentx-native-cohort-probe'])})
        build('clean_generation_conservatively_differs')

        # Exercise actual generator failure before token emission without an
        # entropy shim, production override or another full project build.
        executable = next(path for path in target.glob('debug/build/agentx-native-cohort-probe-*/build-script-build') if path.is_file())
        failed = run([str(executable)], env={'PATH':environment['PATH']}, expect_success=False)
        assert 'cargo::rustc-env=AGENTX_NATIVE_BUILD_ID=' not in failed['stdout']
        records.append({'case':'missing_cargo_inputs_fail_without_default_token', **failed})
        missing = root/'missing-token.rs'
        missing.write_text('const TOKEN: &str = env!("AGENTX_NATIVE_BUILD_ID"); fn main() { println!("{TOKEN}"); }\n')
        failed = run([compiler, str(missing), '--out-dir', str(root)], expect_success=False)
        records.append({'case':'missing_generated_token_fails_compile', **failed})

    receipt = {'production_generator':str(source/'build.rs'), 'generator_sha256':hashlib.sha256(generator).hexdigest(),
               'toolchain':toolchain, 'rustc':version, 'cases':records,
               'limits':['Real Cargo with the production generator in a minimal disposable crate; not repeated full product builds.',
                         'Feature is harness-only; target is the installed host target. No second compiler/toolchain/architecture is installed or tested.',
                         'Same effective toolchain file/compiler/wrapper inputs demonstrate invalidation, not cross-toolchain runtime support.',
                         'Missing Cargo input/token failures exercised; operating-system entropy failure is not injected.',
                         'Final-crate cargo rustc overrides or wrapper replacement in place are outside the supported graph contract.'],
               'cleanup':'disposable Cargo project/target removed'}
    with args.output.open('x') as stream:
        json.dump(receipt, stream, indent=2)
        stream.write('\n')
    print(json.dumps({'receipt':str(args.output), 'cases':len(records), 'generator_sha256':receipt['generator_sha256']}))


if __name__ == '__main__':
    main()
