export interface SearchTag {
  id: string;
  type: string;
  value: string;
  display: string;
}

export interface FilterDef {
  type: string;
  label: string;
  placeholder: string;
}

export interface SearchProvider<T> {
  filters: FilterDef[];
  filterLocal(items: T[], tags: SearchTag[]): T[];
  filterRemote(tags: SearchTag[]): Promise<T[]>;
}
