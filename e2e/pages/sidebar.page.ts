class SidebarPage {
  get graphNav() { return $('[data-testid="nav-graph"]'); }
  get changesNav() { return $('[data-testid="nav-changes"]'); }
  get branchesNav() { return $('[data-testid="nav-branches"]'); }
  get bisectNav() { return $('[data-testid="nav-bisect"]'); }
  get settingsNav() { return $('[data-testid="nav-settings"]'); }

  async navigateTo(view: string) {
    const nav = await $(`[data-testid="nav-${view}"]`);
    await nav.click();
  }
}

export default new SidebarPage();
