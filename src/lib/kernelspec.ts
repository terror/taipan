import { invoke } from '@tauri-apps/api/core';

import type { KernelSelection } from './execution';
import type { KernelDiscovery, Metadata } from './types';

export function discoverKernelspecs(
  metadata: Metadata
): Promise<KernelDiscovery> {
  return invoke<KernelDiscovery>('discover_kernelspecs', { metadata });
}

export function selectKernel(
  name: string | null
): Promise<KernelSelection | null> {
  return invoke('select_kernel', { name });
}
