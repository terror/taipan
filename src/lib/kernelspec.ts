import { invoke } from '@tauri-apps/api/core';

import type { KernelSelection } from './execution';
import type { KernelDiscovery } from './types';

export function discoverKernelspecs(): Promise<KernelDiscovery> {
  return invoke<KernelDiscovery>('discover_kernelspecs');
}

export function selectKernel(
  name: string | null
): Promise<KernelSelection | null> {
  return invoke('select_kernel', { name });
}
