---
"admin-web": patch
"customer-web": patch
"@ru5ty-gate/customer-mobile": minor
---

Move customer-mobile from Ionic and Capacitor to React Native with Expo (SDK 57), and put every dev server on a fixed port in the 4000 range: admin-web now fails instead of changing port (`strictPort`) on 4004, customer-web runs on 4005, and the customer-mobile Metro server runs on 4007.
