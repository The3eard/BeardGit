class GraphPage {
  get canvas() { return $('canvas'); }
  get searchInput() { return $('[data-testid="graph-search"]'); }

  async isVisible() {
    return (await this.canvas).isDisplayed();
  }
}

export default new GraphPage();
