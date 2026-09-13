export type ClientPlatform = 'mobile' | 'windows' | 'desktop';

export function clientPlatform(device: { userAgent: string; platform: string; maxTouchPoints?: number }): ClientPlatform {
  // iPadOS can advertise a Mac user agent in its default desktop browsing mode.
  if (/Android|iPhone|iPad|iPod|Mobile/i.test(device.userAgent) ||
      (/Mac/i.test(device.platform) && (device.maxTouchPoints ?? 0) > 1)) return 'mobile';
  if (/Windows/i.test(device.userAgent) || /^Win/i.test(device.platform)) return 'windows';
  return 'desktop';
}
