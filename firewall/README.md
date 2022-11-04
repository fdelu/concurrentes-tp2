# Firewall

## Propósito

Un firewall cumple la función de mediador entre dos servidores. **Solo debe utilizarse en un ambiente de pruebas**.

Cuando dos servidores se comunican utilizando un `SocketEncoder`, los paquetes se enviarán primero a un proceso
central, llamado Firewall. El Firewall evaluará el conjunto de reglas con que fue configurado, y si el paquete
se encuentra habilitado para enviarse al host de destino, lo reenviará al mismo. De lo contrario, el paquete se
dropea.

Cada vez que un paquete se envía a un destino correctamente (es decir, ninguna regla bloqueó el envío), se notificará
a todas las reglas de dicho evento, pudiendo modificar su estado interno de ser necesario.

<img src="./resources/diagrama_firewall.png">

Ejemplo de comunicación exitosa entre el servidor 1 y 2. Notar que cada elemento del diagrama es un proceso diferente.

<img src="./resources/diagrama_firewall_drop.png">

Ejemplo en el cual un paquete es dropeado por el Firewall. esto se debe a que una de las reglas con el que fue configurado, retornó `true` al ejecutar la función `is_matching`.

## Reglas

### Implementar nueva regla

Se debe implementar el trait `Rule`

```Rust
pub trait Rule {
    fn is_matching(&self, src: SocketAddr, dst: SocketAddr, data: &[u8]) -> bool;

    fn notify_send(&mut self, packet: &Packet);
}
```

### Agregar regla al firewall

```Rust
let firewall = Firewall::new(...)
let rule = CustomRule::new(...)
firewall.add_rule(rule)
```

## Ejecutar firewall

```Rust
firewall.run()
```