#!/bin/bash
# User-space Android SDK + JDK 17 + headless AVD. No sudo. Shared by ZCode's android-emulator plugin
# (ANDROID_PLUGIN_SDK_PATH / ANDROID_HOME) and Ozone's android-mcp. Idempotent; logs to $LOG.
# NOTE: `sdkmanager --licenses` accepts Google's Android SDK license — the operator authorised running an emulator.
set -u
SDK=${ANDROID_HOME:-$HOME/Android/Sdk}; JDK=$HOME/Android/jdk17; API=${API:-30}; ABI=${ABI:-x86_64}; AVD=${AVD:-ozone_api$API}
IMG="system-images;android-$API;default;$ABI"
mkdir -p "$SDK" "$HOME/Android/dl"; cd "$HOME/Android/dl"
echo "[1/5] JDK 17"; if [ ! -x "$JDK/bin/java" ]; then
  curl -sL -o jdk.tgz "https://api.adoptium.net/v3/binary/latest/17/ga/linux/x64/jdk/hotspot/normal/eclipse" && mkdir -p "$JDK" && tar xzf jdk.tgz -C "$JDK" --strip-components=1; fi
export JAVA_HOME=$JDK PATH=$JDK/bin:$PATH; java -version 2>&1 | head -1
echo "[2/5] cmdline-tools"; if [ ! -x "$SDK/cmdline-tools/latest/bin/sdkmanager" ]; then
  curl -sL -o clt.zip https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip && python3 -c "import zipfile;zipfile.ZipFile('clt.zip').extractall('clt')" \
  && mkdir -p "$SDK/cmdline-tools" && rm -rf "$SDK/cmdline-tools/latest" && mv clt/cmdline-tools "$SDK/cmdline-tools/latest" && chmod +x "$SDK"/cmdline-tools/latest/bin/*; fi
SM=$SDK/cmdline-tools/latest/bin/sdkmanager
echo "[3/5] licenses"; yes | $SM --sdk_root="$SDK" --licenses >/dev/null 2>&1; echo done
echo "[4/5] packages ($IMG)"; $SM --sdk_root="$SDK" "platform-tools" "emulator" "$IMG" 2>&1 | tail -3
echo "[5/5] AVD $AVD"; echo no | "$SDK/cmdline-tools/latest/bin/avdmanager" create avd -n "$AVD" -k "$IMG" --force 2>&1 | tail -2
# modest RAM so it coexists with the host + ZCode on a 7.6 GB box
CFG=$HOME/.android/avd/$AVD.avd/config.ini; [ -f "$CFG" ] && sed -i '/^hw.ramSize/d;/^hw.gpu.mode/d;/^disk.dataPartition.size/d' "$CFG" && printf 'hw.ramSize=1536\nhw.gpu.mode=swiftshader_indirect\ndisk.dataPartition.size=2G\n' >> "$CFG"
echo "READY sdk=$SDK avd=$AVD"; "$SDK/emulator/emulator" -list-avds
