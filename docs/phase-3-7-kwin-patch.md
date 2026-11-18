# Phase 3.7 KWin patch

The private patch adds native observation in KWin itself. It does not add a trusted human approval path. The default build option is off, and the component only starts on KWin's virtual output backend. The checked-in [KWin 6.8.3 patch](../experiments/kwin-authority/phase-3-7/kwin-6.8.3-depthos.patch) is applied to the verified upstream release outside the application source tree. No host package, binary, plugin directory or configuration file is modified.

## Exact source and toolchain

The campaign reused the Phase 3.6 package/source provenance chain. Installed Arch `kwin 6.8.3-1` reports `kwin 6.8.3`. The installed executable SHA-256 is `33350d420b7b196c1e1f90ffa8c1280b4b3102d4e4b60e1929bad525be93df5e`, GNU build ID `6eb58cf1f8d875ef625487ec2f404e8cc969c517`. Installed `libkwin.so.6` has SHA-256 `25259a191bc4c0e2e0eae74a5b4ec34b68890564b157884b7c00fc5bea9155d1`, build ID `560f96b8881b8fb467d81762d9719c774a033314`. The cached package build metadata gives PKGBUILD SHA-256 `d6023831666fecbe025107a305f12cca4d2751959e97e8f97a7e3126db013616`, matching the unpatched [Arch recipe](https://gitlab.archlinux.org/archlinux/packaging/packages/kwin/-/blob/6.8.3-1/PKGBUILD). Its [release archive](https://download.kde.org/stable/plasma/6.8.3/kwin-6.8.3.tar.xz) SHA-256 is `fd5cb783f220ea4526a9afee2f72b582c3e87ef7f91b19bc5c7a6662ffe7fbdc`. Phase 3.7 copied and rehashed that archive, package recipe and build metadata into restricted ignored local evidence before extracting. This is source correspondence; the private build is not a reproducible build of the installed distribution package.

Build in an owner-only directory with no install step, using the platform's native headers and privately extracted build dependencies. Launch only under the validated worker. The build and runtime commands and executable identity are in the [validation report](phase-3-7-validation.md). Applying the diff with `git apply` requires being outside the application Git worktree or in a separate source repository. For example:

```sh
tar xf /path/to/verified/kwin-6.8.3.tar.xz -C /path/to/private/source-parent
cd /path/to/private/source-parent/kwin-6.8.3
git init -q
git apply --check /path/to/DepthOS/experiments/kwin-authority/phase-3-7/kwin-6.8.3-depthos.patch
git apply /path/to/DepthOS/experiments/kwin-authority/phase-3-7/kwin-6.8.3-depthos.patch
cmake -S . -B /path/to/private/build -G Ninja -DBUILD_TESTING=OFF \
    -DKWIN_DEPTHOS_AUTHORITY_EXPERIMENT=ON
cmake --build /path/to/private/build --target kwin_wayland eis
```

## Patch structure

- `CMakeLists.txt` switches the experiment on explicitly. `src/CMakeLists.txt` supplies the executable's two private library dependencies when their KWin features are enabled.
- `main_wayland.cpp` attaches after workspace creation and before accepting Wayland clients. It destroys the observer before ordinary plugin, workspace and input teardown.
- `depthos_authority_experiment.h` owns native observation, exact guards, retirement and the isolated built-in fixture token. Its plugin fallback compiles the same observer without the token machinery. The fallback uses the exact installed ABI and remains a separate runtime result.
- `depthos_input_origin.h` assigns observational source labels in backend constructors. The observer consumes them through `InputEventSpy` before KWin's filters and Qt event conversion. It never consumes application input or treats a label as approval.

## Input and trust

[Keyboard and pointer ingress](https://github.com/KDE/kwin/tree/v6.8.3/src) retains an `InputDevice *` through the spy. The annotated libinput category is only a local candidate unless the known virtual-device indicator makes it fake/virtual; a missing sysfs path is unknown. The fake-input device constructor labels its devices fake/virtual. The EIS device constructor labels its devices EIS or portal; at that device layer neither an upstream portal credential nor a physical source is authenticated. Remote/portal versus other EIS, natively remapped accessibility input and any unlabelled device are not reliably distinguishable by this implementation. Ordinary Qt/QML events have no authority ingress. A QObject dynamic property, device pointer or logical seat label is never a trust attestation. The lab has not qualified real device, remote, accessibility or virtual input transitions merely by compiling those hooks.

The component has no QML, Wayland or D-Bus approval method and no source-label-to-token conversion. Native arbitrary code already loaded into KWin, debugger or process-memory access, startup-environment manipulation, and the ability to alter executable, libraries or config are outside the private fixture's trust model. A future production domain must independently constrain all of these. A supported effect loader cannot supply that guarantee. Nothing here treats the existing ordinary Wayland preview as a protected prompt.

## Rollback

Revert the Phase 3.7 application commits on the independent experiment branch or return to the existing Phase 3.6 checkout. Remove only owner-approved disposable builds and fixture outputs. No host package removal, configuration restoration or Git checkpoint repair is part of rollback.
