import { greeting } from "greet";


class Hello extends Component {
  override onStart() {
    console.log(greeting("world", 1));
  }

  override onUpdate(_dt: number) {
    console.log("tick");
  }
}

register({ Hello });
