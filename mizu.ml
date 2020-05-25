open SCaml

type action =
  | Post of bytes list * nat list
  | Poke of address

type message =
  { content : bytes
  ; timestamp : timestamp
  }

type user_data =
  { postal_box : message list
  ; pokes : address list
  }

type storage = (address, user_data) big_map

let post (add : bytes list) (remove : nat list) (storage : storage) =
  let sender = Global.get_sender () in
  let timestamp = Global.get_now () in
  let new_messages = List.map (fun content -> { content; timestamp }) add in
  let new_user_data =
    match BigMap.get sender storage with
    | None ->
      assert (List.length remove = Nat 0);
      { postal_box = new_messages; pokes = [] }
    | Some user_data ->
      let _, remaining_indices, postal_box =
        List.fold_left
          (fun (index, rm_indices, accum) element ->
            match rm_indices with
            | [] -> index +^ Nat 1, [], element :: accum
            | x :: xs when index = x -> index +^ Nat 1, xs, accum
            | _ -> index +^ Nat 1, rm_indices, element :: accum)
          (Nat 0, remove, new_messages)
          user_data.postal_box
      in
      assert (List.length remaining_indices = Nat 0);
      { user_data with postal_box }
  in
  ([] : operation list), BigMap.update sender (Some new_user_data) storage
;;

let poke (address : address) (storage : storage) =
  match BigMap.get address storage with
  | None -> failwith "invalid address"
  | Some user_data ->
    let new_user_data =
      { user_data with pokes = Global.get_sender () :: user_data.pokes }
    in
    ([] : operation list), BigMap.update address (Some new_user_data) storage
;;

let[@entry] main action storage =
  match action with
  | Post (add, remove) -> post add remove storage
  | Poke address -> poke address storage
;;
