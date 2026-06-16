# Command List

- Get
  - Memory *
  - Disk *
  - Volume *
  - Environment *
  - Group *
  - User *
  - NetworkInterface *
  - NetworkAdapter *
  - Process *
  - Processor *
  - VideoDevice *
  - VideoConfiguration *
  - Battery *
  - Chassis *
  - BIOS *
  - OperatingSystem *
  - Package *
  - Provider *
  - Service (or Daemon) *
  - ScheduledJob (or Crontab) *
  - TimeZone *
  - Bluetooth *
  - USB *
  - CinnamonDesktop *
  - CinnamonApplet *
  - CinnamonExtension *
  - CinnamonActions *

## Future Enhancements

- Additions:
  - all commands: --compress (for use with --json), as well as -jc (combined)
  - set-environment: (merge from json file, only create environment variables that are missing)
- Updates:
  - get-scheduled-job: include actual crontab in output
  - get-operating-system: include Ubuntu version info when distro is Ubuntu based