import { Tabs } from "expo-router";

export default function TabLayout() {
  return (
    <Tabs screenOptions={{ headerShown: true }}>
      <Tabs.Screen name="index" options={{ title: "Order" }} />
      <Tabs.Screen name="connect" options={{ title: "Connect" }} />
    </Tabs>
  );
}
