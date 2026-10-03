// Root layout — react-native-get-random-values MUST load before anything
// that touches @noble/ed25519 (deviceIdentity.ts), so it's imported first,
// here, before any other app code runs.
import "react-native-get-random-values";
import { Stack } from "expo-router";

export default function RootLayout() {
  return <Stack screenOptions={{ headerShown: false }} />;
}
