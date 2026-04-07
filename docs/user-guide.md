# User Guide

This guide collects the everyday commands for using Cron Drop after installation.

## Core Commands

Initialize config:

```bash
crondrop init
```

Start the daemon:

```bash
crondrop start
```

`run` is an alias for `start`:

```bash
crondrop run
```

Stop the daemon:

```bash
crondrop stop
```

Restart the daemon:

```bash
crondrop restart
```

Show current status:

```bash
crondrop status
```

## Schedule Commands

Repeat every hour:

```bash
crondrop schedule every 1h
```

Repeat every 30 minutes during a time window:

```bash
crondrop schedule every 30m --from 08:00 --to 22:00
```

Use weekdays only:

```bash
crondrop schedule every 1h --from 08:00 --to 18:00 --weekdays-only
```

Set fixed reminder times:

```bash
crondrop schedule add --at 09:00 --at 13:00 --at 18:00
```

Save the schedule without starting the app:

```bash
crondrop schedule every 1h --no-start
```

## Popup And Tray

Open the popup immediately to preview the reminder UI:

```bash
crondrop preview
```

The following aliases also work:

```bash
crondrop popup
crondrop show-popup
crondrop test
```

Start the tray manually:

```bash
crondrop tray
```

## Pause And Resume

Pause reminders for the rest of today:

```bash
crondrop pause --today
```

Resume reminders:

```bash
crondrop resume
```

## Config And Theme

Show the active config:

```bash
crondrop config show
```

Print the config file path:

```bash
crondrop config path
```

Change the popup theme:

```bash
crondrop theme cozy
```

## Autostart

Install launch-at-login:

```bash
crondrop autostart install
```

Check autostart status:

```bash
crondrop autostart status
```

Remove launch-at-login:

```bash
crondrop autostart remove
```
