// SPDX-License-Identifier: Apache-2.0
/**
 * Bounded least-recently-used cache for graph row chunks (PERF-03).
 * Reads promote an entry; inserting past `max` drops the least recently used one.
 */
export class ChunkCache<V> {
  private readonly map = new Map<string, V>();

  constructor(private readonly max: number) {
    if (!(max >= 1)) throw new RangeError("ChunkCache needs max >= 1");
  }

  get size(): number {
    return this.map.size;
  }

  has(key: string): boolean {
    return this.map.has(key);
  }

  get(key: string): V | undefined {
    const value = this.map.get(key);
    if (value !== undefined) {
      // Re-insert to move the key to the most-recent end of the Map's insertion order.
      this.map.delete(key);
      this.map.set(key, value);
    }
    return value;
  }

  set(key: string, value: V): void {
    this.map.delete(key);
    this.map.set(key, value);
    while (this.map.size > this.max) {
      const oldest = this.map.keys().next().value as string;
      this.map.delete(oldest);
    }
  }

  deleteWhere(predicate: (key: string) => boolean): void {
    for (const key of [...this.map.keys()]) if (predicate(key)) this.map.delete(key);
  }

  clear(): void {
    this.map.clear();
  }
}
