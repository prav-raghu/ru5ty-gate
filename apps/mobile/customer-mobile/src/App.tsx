import { StatusBar } from "expo-status-bar";
import { StyleSheet, Text, View } from "react-native";

import { API_BASE_URL } from "./constants/config";

export default function App() {
    return (
        <View style={styles.container}>
            <Text style={styles.title}>Customer Mobile</Text>
            <Text>API: {API_BASE_URL}</Text>
            <StatusBar style="auto" />
        </View>
    );
}

const styles = StyleSheet.create({
    container: {
        flex: 1,
        backgroundColor: "#fff",
        alignItems: "center",
        justifyContent: "center",
    },
    title: {
        fontSize: 20,
        fontWeight: "600",
        marginBottom: 8,
    },
});
