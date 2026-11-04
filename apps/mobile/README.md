# mobile

> **Status: planned, not implemented.** Design from [`docs/architecture.md`](../../docs/architecture.md). Update this file when work starts.

**Tech:** React Native (Expo)

## Role
The mobile app: chat with the assistant, remote control, notifications, camera view, presence.

## Planned design
- At home: talks to the gateway directly over the LAN.
- Away from home: goes through **AWS IoT Core**, signed in with Cognito. The box keeps one outbound connection, so
  nothing is opened on the router.
- Push notifications through Amazon SNS (FCM / APNs).

## Interfaces
- Gateway HTTP API (same as the web app, see `apps/gateway/docs/API.md`).

## Depends on
gateway, `infra/aws` (Cognito, IoT Core, SNS).
