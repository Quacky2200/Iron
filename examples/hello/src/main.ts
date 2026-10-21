
class Hello extends Component {
  override onStart() {
    console.log("hello");
  }

  override onUpdate(_dt: number) {
    console.log("tick");
  }
}

register({ Hello });
